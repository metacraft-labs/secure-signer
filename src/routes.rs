use crate::route_handlers::{
    KeyImportRequest, RemoteAttestationRequest, epid_remote_attestation_service, eth_key_gen_service, 
    list_eth_keys_service, bls_key_gen_service, list_generated_bls_keys_service, 
    bls_key_import_service, list_imported_bls_keys_service, secure_sign_bls, 
};
use warp::Filter;
use warp::{http::StatusCode, reply};


/// Signs off on validator duty.
/// https://consensys.github.io/web3signer/web3signer-eth2.html#tag/Signing
pub fn bls_sign_route() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path("api"))
        .and(warp::path("v1"))
        .and(warp::path("eth2"))
        .and(warp::path("sign"))
        .and(warp::path::param())
        .and(warp::body::bytes())
        .and_then(secure_sign_bls)
}

/// Returns a 200 status code if server is alive
pub fn upcheck_route() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::get()
        .and(warp::path("upcheck"))
        .and(warp::any().map(warp::reply))
}

/// Imports a BLS private key to the Enclave. 
/// https://consensys.github.io/web3signer/web3signer-eth2.html#tag/Keymanager
pub fn bls_key_import_route() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path("eth"))
        .and(warp::path("v1"))
        .and(warp::path("keystores"))
        .and(warp::body::json::<KeyImportRequest>())
        .and_then(bls_key_import_service)
}

/// Returns all hex-encoded BLS public keys, where the private keys were imported and saved in the Enclave.
/// https://consensys.github.io/web3signer/web3signer-eth2.html#tag/Keymanager
pub fn list_imported_bls_keys_route() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::get()
        .and(warp::path("eth"))
        .and(warp::path("v1"))
        .and(warp::path("keystores"))
        .and_then(list_imported_bls_keys_service)
}

/// Performs EPID remote attestation, committing to a public key (SECP256k1 or BLS) if the corresponding
/// private key is safeguarded by the enclave.
/// Route added by Secure-Signer
pub fn epid_remote_attestation_route() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path("eth"))
        .and(warp::path("v1"))
        .and(warp::path("remote-attestation"))
        .and(warp::path::param())
        .and_then(epid_remote_attestation_service)
}

/// Generates a new ETH (SECP256K1) private key in Enclave. The ETH public key is returned 
/// Route added by Secure-Signer
pub fn eth_key_gen_route() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path("eth"))
        .and(warp::path("v1"))
        .and(warp::path("keygen"))
        .and(warp::path("secp256k1"))
        .and_then(eth_key_gen_service)
}

/// Returns all hex-encoded ETH public keys, where the private keys were generated and saved in the Enclave.
/// Route added by Secure-Signer
pub fn list_generated_eth_keys_route() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::get()
        .and(warp::path("eth"))
        .and(warp::path("v1"))
        .and(warp::path("keygen"))
        .and(warp::path("secp256k1"))
        .and_then(list_eth_keys_service)
}

/// Generates a new BLS private key in Enclave. 
/// Route added by Secure-Signer
pub fn bls_key_gen_route() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path("eth"))
        .and(warp::path("v1"))
        .and(warp::path("keygen"))
        .and(warp::path("bls"))
        .and_then(bls_key_gen_service)
}

/// Returns all hex-encoded BLS public keys, where the private keys were generated and saved in the Enclave.
/// Route added by Secure-Signer
pub fn list_generated_bls_keys_route() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::get()
        .and(warp::path("eth"))
        .and(warp::path("v1"))
        .and(warp::path("keygen"))
        .and(warp::path("bls"))
        .and_then(list_generated_bls_keys_service)
}

#[cfg(test)]
pub mod api_signing_tests {
    use super::*;
    use std::fs;
    use serde_json;
    use crate::eth_signing::slash_resistance_tests::*;
    use crate::route_handlers::{mock_requests::*, SecureSignerSig};
    use crate::eth_types::{RandaoRevealRequest, BlockV2Request};

    pub async fn mock_secure_sign_bls_route(bls_pk: &String, json_req: &String) -> warp::http::Response<bytes::Bytes> {
        let filter = bls_sign_route();
        let uri = format!("/api/v1/eth2/sign/{}", bls_pk);

        println!("mocking request to: {uri}");
        let res = warp::test::request()
            .method("POST")
            .path(&uri)
            .body(&json_req)
            .reply(&filter)
            .await;
        res
    }

    #[tokio::test]
    async fn test_bls_sign_route_block_type() {
        // clear state
        fs::remove_dir_all("./etc");

        // new keypair
        let bls_pk_hex = setup_keypair();

        // mock data for a BLOCK request
        let json_req = mock_propose_block_request("10");
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        assert_eq!(resp.status(), 200);

        // mock data for a BLOCK request (attempt a slashable offense - non-increasing slot)
        let json_req = mock_propose_block_request("10");
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        assert_eq!(resp.status(), 412);

        // mock data for a BLOCK request (attempt a slashable offense - decreasing slot)
        let json_req = mock_propose_block_request("9");
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        assert_eq!(resp.status(), 412);

        // mock data for a BLOCK request 
        let json_req = mock_propose_block_request("11");
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn test_bls_sign_route_block_v2_type() {
        // clear state
        fs::remove_dir_all("./etc");

        // new keypair
        let bls_pk_hex = setup_keypair();

        // mock data for a BLOCK request
        let json_req = mock_block_v2_bellatrix_request("10");
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        assert_eq!(resp.status(), 200);

        // mock data for a BLOCK request (attempt a slashable offense - non-increasing slot)
        let json_req = mock_block_v2_bellatrix_request("10");
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        assert_eq!(resp.status(), 412);

        // mock data for a BLOCK request (attempt a slashable offense - decreasing slot)
        let json_req = mock_block_v2_bellatrix_request("9");
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        assert_eq!(resp.status(), 412);

        // mock data for a BLOCK request 
        let json_req = mock_block_v2_bellatrix_request("11");
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn test_bls_sign_route_attestation_type() {
        // clear state
        fs::remove_dir_all("./etc");

        // new keypair
        let bls_pk_hex = setup_keypair();

        // mock data for ATTESTATION request
        let json_req = mock_attestation_request("10", "11");
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        assert_eq!(resp.status(), 200);

        // mock data for ATTESTATION request (attempt a slashable offense - decreasing source)
        let json_req = mock_attestation_request("0", "12");
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        assert_eq!(resp.status(), 412);

        // mock data for ATTESTATION request (attempt a slashable offense - non-increasing target)
        let json_req = mock_attestation_request("10", "11");
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        assert_eq!(resp.status(), 412);

        // mock data for ATTESTATION request (non-increasing source + increasing target)
        let json_req = mock_attestation_request("10", "12");
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        assert_eq!(resp.status(), 200);

        // mock data for ATTESTATION request (increasing source + increasing target)
        let json_req = mock_attestation_request("11", "13");
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn test_bls_sign_route_randao_reveal_type() {
        // clear state
        fs::remove_dir_all("./etc");

        // new keypair
        let bls_pk_hex = setup_keypair();

        // mock data for RANDAO_REVEAL request
        let json_req = mock_randao_reveal_request();
        let parsed_req: RandaoRevealRequest = serde_json::from_str(&json_req).unwrap();
        assert_eq!(parsed_req.fork_info.fork.previous_version, [0,0,0,0]);
        assert_eq!(parsed_req.fork_info.fork.current_version, [0,0,0,0]);
        assert_eq!(parsed_req.fork_info.fork.epoch, 0);
        assert_eq!(parsed_req.fork_info.genesis_validators_root, [42_u8; 32]);
        assert_eq!(parsed_req.signingRoot, [191, 112, 219, 187, 200, 50, 153, 251, 135, 115, 52, 234, 234, 239, 179, 45, 244, 66, 66, 193, 191, 7, 140, 220, 24, 54, 220, 195, 40, 45, 79, 189]);
        assert_eq!(parsed_req.randao_reveal.epoch, 0);

        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        println!("{:?}", resp);
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn test_bls_sign_route_aggregate_and_proof_type() {
        // clear state
        fs::remove_dir_all("./etc");

        // new keypair
        let bls_pk_hex = setup_keypair();

        // mock data for RANDAO_REVEAL request
        let json_req = mock_aggregate_and_proof_request("0", "1");
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        println!("{:?}", resp);
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn test_bls_sign_route_block_V2_bellatrix_type() {
        // clear state
        fs::remove_dir_all("./etc");

        // new keypair
        let bls_pk_hex = setup_keypair();

        // mock data for RANDAO_REVEAL request
        let json_req = mock_block_v2_bellatrix_request("24000");
        let parsed_req: BlockV2Request = serde_json::from_str(&json_req).unwrap();
        assert_eq!(parsed_req.fork_info.fork.previous_version, [128,0,0,112]);
        assert_eq!(parsed_req.fork_info.fork.current_version, [128,0,0,113]);
        assert_eq!(parsed_req.fork_info.fork.epoch, 750);
        assert_eq!(parsed_req.fork_info.genesis_validators_root, [42_u8; 32]);
        assert_eq!(parsed_req.signingRoot, [46, 191, 194, 215, 9, 68, 204, 47, 191, 246, 214, 124, 109, 156, 187, 4, 61, 127, 190, 10, 102, 13, 36, 139, 110, 102, 108, 225, 16, 175, 65, 138]);
        assert_eq!(parsed_req.beacon_block.block_header.slot, 24000);
        assert_eq!(parsed_req.beacon_block.block_header.proposer_index, 0);
        assert_eq!(parsed_req.beacon_block.block_header.parent_root, [0_u8; 32]);
        assert_eq!(parsed_req.beacon_block.block_header.state_root, [0_u8; 32]);
        assert_eq!(parsed_req.beacon_block.block_header.body_root, [205, 124, 73, 150, 110, 190, 114, 177, 33, 78, 109, 71, 51, 173, 246, 191, 6, 147, 92, 95, 188, 59, 58, 208, 142, 132, 227, 8, 84, 40, 184, 47]);

        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        println!("{:?}", resp);
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn test_bls_sign_deposit_type() {
        // clear state
        fs::remove_dir_all("./etc");

        // new keypair
        let bls_pk_hex = setup_keypair();

        // mock data for DEPOSIT request
        let json_req = mock_deposit_request();
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        println!("{:?}", resp);
        assert_eq!(resp.status(), 200);
    }


    #[tokio::test]
    async fn test_bls_sign_validator_registration_altair_type() {
        // clear state
        fs::remove_dir_all("./etc");

        let json_req = format!(r#"
        {{
            "type": "VALIDATOR_REGISTRATION",
            "signingRoot": "0xbaeddafaf70f1699e5abbafa1a15bb807de4f2c889b4e59be1ef62e23f1206a8",
            "validator_registration": {{
                "fee_recipient": "0x2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a",
                "gas_limit": "30000000",
                "timestamp":"100",
                "pubkey": "0x8349434ad0700e79be65c0c7043945df426bd6d7e288c16671df69d822344f1b0ce8de80360a50550ad782b68035cb18"
            }}
        }}"#);

        // new keypair
        let bls_pk_hex = setup_keypair();

        // mock data for RANDAO_REVEAL request
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        println!("{:?}", resp);
        assert_eq!(resp.status(), 200);
        let sig: SecureSignerSig = serde_json::from_slice(resp.body()).unwrap();
        // assert_eq!(sig.signature, "0x94dc67c0ada5effb5fbca4a29f48a4805df96b4e05418dc7846d73aceb0d1800cb1a6100410da7fecf42798a2e3ab0620abf3ef7aed5987970294fe3ad84995a232d3d0758f44be362a4351666e01b6444146f26d19ebc7e30d1534e0f702d9b".to_string());


    }

    #[tokio::test]
    async fn test_bls_sign_validator_registration_type_mevboost_test() {
        // clear state
        fs::remove_dir_all("./etc");

        let json_req = format!(r#"
        {{
            "type": "VALIDATOR_REGISTRATION",
            "signingRoot": "0xbaeddafaf70f1699e5abbafa1a15bb807de4f2c889b4e59be1ef62e23f1206a8",
            "validator_registration": {{
                "fee_recipient": "0xdb65fEd33dc262Fe09D9a2Ba8F80b329BA25f941",
                "gas_limit": "278234191203",
                "timestamp":"1234356",
                "pubkey": "0x8a1d7b8dd64e0aafe7ea7b6c95065c9364cf99d38470c12ee807d55f7de1529ad29ce2c422e0b65e3d5a05c02caca249"
            }}
        }}"#);

        // new keypair
        let bls_pk_hex = setup_keypair2();

        // mock data for RANDAO_REVEAL request
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        println!("{:?}", resp);
        assert_eq!(resp.status(), 200);
        let sig: SecureSignerSig = serde_json::from_slice(resp.body()).unwrap();
        // assert_eq!(sig.signature, "0x8209b5391cd69f392b1f02dbc03bab61f574bb6bb54bf87b59e2a85bdc0756f7db6a71ce1b41b727a1f46ccc77b213bf0df1426177b5b29926b39956114421eaa36ec4602969f6f6370a44de44a6bce6dae2136e5fb594cce2a476354264d1ea".to_string());


    }

    
    // todo
    // async fn test_bls_sign_route_aggregation_slot_type() {}
    // async fn test_bls_sign_route_sync_committee_message_type() {}
    // async fn test_bls_sign_route_sync_committee_selection_proof_type() {}
    // async fn test_bls_sign_route_sync_committee_contribution_and_proof_type() {}

}


#[cfg(test)]
mod key_management_tests {
    use super::*;
    use crate::keys::{new_bls_key, new_eth_key, CIPHER_SUITE, eth_pk_from_hex, new_keystore};
    use crate::remote_attestation::{AttestationEvidence};
    use crate::slash_protection::test_slash_protection::dummy_slash_protection_data;
    use crate::routes::*;
    use crate::route_handlers::*;
    use crate::route_handlers::{mock_requests::*, SecureSignerSig};
    use crate::routes::api_signing_tests::*;
    use ecies::{decrypt, encrypt};
    use blst::min_pk::{SecretKey, PublicKey, Signature};
    use ecies::PublicKey as EthPublicKey;
    use ecies::SecretKey as EthSecretKey;
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;  
    use serde_json;

    fn dummy_keystore_string() -> String {
        r#"{"crypto":{"cipher":"aes-128-ctr","cipherparams":{"iv":"dc4104fe6715cbf0c6cccc2f8aefa68b"},"ciphertext":"b24066c307ff23754aa12c0d5ef33527514de717157aaf67494a36290e25ce47","kdf":"scrypt","kdfparams":{"dklen":32,"n":8192,"p":1,"r":8,"salt":"1f313d2b1ed0f85bec298478e28772f132412696b97049da55be982da6da8cbc"},"mac":"b8586806b1271a4bf63977113bd44a131bafa07aa431e849de4eb79fecc289e9"},"id":"33959655-ff04-41ea-b60b-51bef65eb1ec","version":3}"#.to_string()
    }

    async fn call_eth_key_gen_route() -> KeyGenResponse {
        let filter = eth_key_gen_route();

        // mock the request
        let res = warp::test::request()
            .method("POST")
            .path("/eth/v1/keygen/secp256k1")
            .reply(&filter)
            .await;

        assert_eq!(res.status(), 200);

        // parse the resp
        let resp: KeyGenResponse = serde_json::from_slice(&res.body()).unwrap();
        resp
    }

    async fn mock_request_eth_key_list_route() -> ListKeysResponse {
        let filter = list_generated_eth_keys_route();
        let res = warp::test::request()
            .method("GET")
            .path("/eth/v1/keygen/secp256k1")
            .reply(&filter)
            .await;

        assert_eq!(res.status(), 200);
        let resp: ListKeysResponse = serde_json::from_slice(&res.body()).unwrap();
        resp
    }

    #[tokio::test]
    async fn test_call_eth_key_gen_route() {
        fs::remove_dir_all("./etc");
        let resp = call_eth_key_gen_route().await;
        println!("resp: {:?}", resp);

        let list_keys_resp = mock_request_eth_key_list_route().await;
        println!("resp: {:?}", list_keys_resp);
        assert_eq!(list_keys_resp.data.len(), 1);
        assert_eq!(list_keys_resp.data[0].pubkey, resp.pk_hex);
    }

    async fn mock_request_bls_key_import_route(slash_protection: Option<String>) -> KeyImportResponse {
        // 1) generate ETH secret key in enclave
        let resp = call_eth_key_gen_route().await;
        let enclave_eth_pk_hex = resp.pk_hex;
        let eth_pk = eth_pk_from_hex(enclave_eth_pk_hex.clone()).unwrap();
        let enclave_eth_pk_bytes = eth_pk.serialize_compressed();

        // expected to be done by client
        // 2) request enclave to do remote attestation
        // 3) verify evidence
        // 4) extract ETH pub key

        // 5) BLS key to import
        let password = "";
        let bls_pk_hex = "0x8349434ad0700e79be65c0c7043945df426bd6d7e288c16671df69d822344f1b0ce8de80360a50550ad782b68035cb18".to_string();
        let keystore_str = dummy_keystore_string();

        // 6) encrypt BLS key with ETH pub key
        let ct_password = encrypt(&enclave_eth_pk_bytes, password.as_bytes()).unwrap();
        let ct_password_hex = hex::encode(ct_password);

        // 7) make payload to send /eth/v1/keystores POST request
        let req = KeyImportRequest {
            keystore: keystore_str,
            ct_password_hex: ct_password_hex,
            encrypting_pk_hex: enclave_eth_pk_hex,
            slashing_protection: slash_protection,
        };
        println!("making bls key import req: {:?}", req);

        // 8) make the actual request
        let filter = bls_key_import_route();
        let res = warp::test::request()
            .method("POST")
            .header("accept", "application/json")
            .path("/eth/v1/keystores")
            .json(&req)
            .reply(&filter)
            .await;


        println!("{:?}", res.body());
        assert_eq!(res.status(), 200);

        let resp: KeyImportResponse = serde_json::from_slice(&res.body()).unwrap();

        assert_eq!(resp.data[0].status, "imported".to_string());
        assert_eq!(resp.data[0].message, bls_pk_hex);
        resp
    }


    #[tokio::test]
    async fn test_request_bls_key_import_route() {
        fs::remove_dir_all("./etc");
        let sp = None;
        let resp = mock_request_bls_key_import_route(sp).await;
        println!("{:?}", resp);
    }

    #[tokio::test]
    async fn test_request_bls_key_import_route_with_slash_protection() {
        fs::remove_dir_all("./etc");
        let sp = dummy_slash_protection_data();
        let resp = mock_request_bls_key_import_route(Some(sp)).await;
        println!("{:?}", resp);


    }

    #[tokio::test]
    async fn test_request_bls_key_import_route_with_slash_protection_and_try_slashing() {
        fs::remove_dir_all("./etc");
        let sp = dummy_slash_protection_data();
        let resp = mock_request_bls_key_import_route(Some(sp)).await;
        let bls_pk_hex = &resp.data.first().unwrap().message;

        // from dummy slash protection:
        let last_slot = 81952;
        let last_source_epoch = 2290;
        let last_target_epoch = 3008;

        // mock data for ATTESTATION request (attempt a slashable offense - decreasing source)
        let json_req = mock_attestation_request(&(last_source_epoch - 1).to_string(), &(last_target_epoch + 100).to_string());
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        assert_eq!(resp.status(), 412);

        // mock data for ATTESTATION request (attempt a slashable offense - non-increasing target)
        let json_req = mock_attestation_request(&last_source_epoch.to_string(), &last_target_epoch.to_string());
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        assert_eq!(resp.status(), 412);

        // mock data for ATTESTATION request - should be 200
        let json_req = mock_attestation_request(&last_source_epoch.to_string(), &(last_target_epoch + 1).to_string());
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        assert_eq!(resp.status(), 200);

        // mock data for a BLOCK request (attempt a slashable offense - non-increasing slot)
        let json_req = mock_propose_block_request(&last_slot.to_string());
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        assert_eq!(resp.status(), 412);

        // mock data for a BLOCK request (attempt a slashable offense - decreasing slot)
        let json_req = mock_propose_block_request(&(last_slot - 1).to_string());
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        assert_eq!(resp.status(), 412);

        // mock data for a BLOCK request - should be 200
        let json_req = mock_propose_block_request(&(last_slot + 1).to_string());
        let resp = mock_secure_sign_bls_route(&bls_pk_hex, &json_req).await;
        assert_eq!(resp.status(), 200);

    }

    async fn mock_request_bls_key_list_route() -> ListKeysResponse {
        let filter = list_imported_bls_keys_route();
        let res = warp::test::request()
            .method("GET")
            .path("/eth/v1/keystores")
            .reply(&filter)
            .await;

        assert_eq!(res.status(), 200);
        let resp: ListKeysResponse = serde_json::from_slice(&res.body()).unwrap();
        resp
    }

    #[tokio::test]
    async fn test_list_imported_bls_keys_route() {
        // clear any existing local keys
        fs::remove_dir_all("./etc");
        let key_gen_resp = mock_request_bls_key_import_route(None).await;
        println!("key_gen_resp {:?}", key_gen_resp);
        let bls_pk_hex = key_gen_resp.data[0].message.clone();
        assert_eq!(key_gen_resp.data.len(), 1);

        let list_keys_resp = mock_request_bls_key_list_route().await;
        assert_eq!(list_keys_resp.data.len(), 1);
        assert_eq!(list_keys_resp.data[0].pubkey, bls_pk_hex);
    }


    async fn mock_request_bls_key_gen_route() -> KeyGenResponse {
        let filter = bls_key_gen_route();
        let res = warp::test::request()
            .method("POST")
            .path("/eth/v1/keygen/bls")
            .reply(&filter)
            .await;

        println!{"{:?}", res.body()};
        assert_eq!(res.status(), 200);

        let resp: KeyGenResponse = serde_json::from_slice(&res.body()).unwrap();

        resp
    }

    async fn mock_request_generated_bls_key_list_route() -> ListKeysResponse {
        let filter = list_generated_bls_keys_route();
        let res = warp::test::request()
            .method("GET")
            .path("/eth/v1/keygen/bls")
            .reply(&filter)
            .await;

        assert_eq!(res.status(), 200);
        let resp: ListKeysResponse = serde_json::from_slice(&res.body()).unwrap();
        resp
    }

    use crate::slash_protection::SlashingProtectionData;

    #[tokio::test]
    async fn test_bls_key_gen_route() {
        // clear any existing local keys
        fs::remove_dir_all("./etc");
        let key_gen_resp = mock_request_bls_key_gen_route().await;
        let bls_pk_hex = key_gen_resp.pk_hex.clone();

        // verify route generated a SlashingProtectionData .json
        let db = SlashingProtectionData::read(&bls_pk_hex).unwrap();
        assert_eq!(db.signed_blocks.len(), 0);
        assert_eq!(db.signed_attestations.len(), 0);

        let list_keys_resp = mock_request_generated_bls_key_list_route().await;
        assert_eq!(list_keys_resp.data.len(), 1);
        assert_eq!(list_keys_resp.data[0].pubkey, bls_pk_hex);
    }
}
