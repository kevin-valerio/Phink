use contract_transcode::ContractMessageTranscoder;
use serde::ser::Error;
use serde::Deserialize;
use serde_json::Value;
use sp_core::H256;
use sp_io::hashing::sha2_256;
use std::fs;
use std::hash::Hash;
use std::path::Path;

pub type Selector = [u8; 4];

#[derive(Default, Clone)]
pub struct PayloadCrafter {}

impl PayloadCrafter {
    /// Extract all selectors for a given spec
    /// Parses a JSON and returns a list of all possibles messages
    /// # Argument
    ///
    /// * `json_data`: The JSON metadata of the smart-contract
    ///
    /// returns: Vec<Selector>
    ///
    /// # Examples
    ///
    /// ```
    /// PayloadCrafter::extract(flipper_specs)
    /// ```

    pub fn extract_all(json_data: &String) -> Vec<Selector> {
        #[derive(Deserialize)]
        struct Spec {
            constructors: Vec<SelectorEntry>,
            messages: Vec<SelectorEntry>,
        }

        #[derive(Deserialize)]
        struct SelectorEntry {
            selector: String,
        }

        let v: Value = serde_json::from_str(json_data.as_str()).unwrap();

        let spec: Spec = serde_json::from_value(v["spec"].clone()).unwrap();

        let mut selectors: Vec<Selector> = Vec::new();
        for entry in spec.constructors.iter().chain(spec.messages.iter()) {
            let bytes: Vec<u8> = hex::decode(&entry.selector.trim_start_matches("0x"))
                .unwrap()
                .try_into()
                .map_err(|_| serde_json::Error::custom("Selector is not a valid 4-byte array"))
                .unwrap();
            selectors.push(<[u8; 4]>::try_from(bytes).unwrap());
        }
        selectors
    }

    pub fn extract_invariants(json_data: &str) -> Vec<Selector> {
        let data: Value = serde_json::from_str(json_data)
            .expect("JSON was not well-formatted");

        data["spec"]["messages"].as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|message| {
                message["label"].as_str()
                    .filter(|label| label.starts_with("phink_"))
                    .and_then(|_| message["selector"].as_str())
                    .and_then(|selector| Some(decode_selector(selector)))
            })
            .collect()
    }


    /// Return the smart-contract constructor based on its spec. If there are multiple constructors,
    /// returns the one that preferably doesn't have args. If no suitable constructor is found or there
    /// is an error in processing, this function returns `None`.
    pub fn get_constructor(json_data: &String) -> Option<[u8; 4]> {
        // Parse the JSON data safely, return None if parsing fails.
        let parsed_json: Value = match serde_json::from_str(&json_data) {
            Ok(data) => data,
            Err(_) => return None,
        };

        // Access the constructors array, return None if it's not found or not an array.
        let constructors = parsed_json["spec"]["constructors"].as_array()?;

        // If there is exactly one constructor, return its selector if available.
        if constructors.len() == 1 {
            return get_selector_bytes(constructors[0]["selector"].as_str()?);
        }

        // Otherwise, look for a constructor without arguments.
        for constructor in constructors {
            if constructor["args"].as_array().map_or(false, Vec::is_empty) {
                return get_selector_bytes(constructor["selector"].as_str()?);
            }
        }

        // Return None if no suitable constructor is found.
        None
    }
}

/// Encode `Strong("0xbabe")` to a proper `Selector`
fn decode_selector(encoded: &str) -> Selector {
    let bytes: Vec<u8> = hex::decode(encoded.trim_start_matches("0x")).unwrap();
    <[u8; 4]>::try_from(bytes).expect("Selector is not a valid 4-byte array")
}

/// Helper function to decode a hexadecimal string selector into a byte array of length 4.
/// Returns `None` if the decoding or conversion fails.
fn get_selector_bytes(selector_str: &str) -> Option<Selector> {
    hex::decode(selector_str.trim_start_matches("0x"))
        .ok()?
        .try_into()
        .ok()
}

/// A simple helper used to directly encode a message i.e. `flip` to a proper selector `[u8; 4]`
#[macro_export]
macro_rules! message_to_bytes {
    ($s:expr) => {{
        let hash = blake2_256($s.as_bytes());
        [hash[0], hash[1], hash[2], hash[3]]
    }};
}

#[test]
fn fetch_correct_dns_invariant() {
    let flipper_specs = fs::read_to_string("sample/dns/target/ink/dns.json").unwrap();

    let extracted: String = PayloadCrafter::extract_invariants(&flipper_specs)
        .iter()
        .map(|x| hex::encode(x) + " ")
        .collect();

    assert_eq!(extracted, "2e15cab0 5d17ca7f ");
}

#[test]
fn fetch_correct_flipper_selectors() {
    let flipper_specs = fs::read_to_string("sample/flipper/target/ink/flipper.json").unwrap();
    let extracted: String = PayloadCrafter::extract_all(&flipper_specs)
        .iter()
        .map(|x| hex::encode(x) + " ")
        .collect();

    // Flipper default selectors
    assert_eq!(extracted, "9bae9d5e ed4b9d1b 633aa551 2f865bd9 ");
}

#[test]
fn fetch_correct_dns_constructor() {
    let dns_spec = fs::read_to_string("sample/dns/target/ink/dns.json").unwrap();
    let ctor: Selector = PayloadCrafter::get_constructor(&dns_spec).unwrap();

    // DNS default selectors
    assert_eq!(hex::encode(ctor), "9bae9d5e");
}

#[test]
fn encode_works_good() {
    let metadata_path = Path::new("sample/dns/target/ink/dns.json");
    let transcoder = ContractMessageTranscoder::load(metadata_path).unwrap();
    let constructor = "set_address";
    let args = [
        //name: Hash, new_address: AccountId
        "re",
        "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
    ];
    let data = transcoder.encode(&constructor, args).unwrap();
    let hex = hex::encode(data);
    println!("Encoded constructor data {}", hex);
    assert!(!hex.is_empty())
}

#[test]
fn decode_works_good() {
    let metadata_path = Path::new("sample/dns/target/ink/dns.json");
    let transcoder = ContractMessageTranscoder::load(metadata_path).unwrap();

    let encoded_bytes =
        hex::decode("229b553f9400000000000000000027272727272727272700002727272727272727272727")
            .unwrap();
    let hex = transcoder.decode_contract_message(&mut &encoded_bytes[..]);
    assert!(hex.is_ok());
    println!("{:?}", hex);
}

#[test]
fn basic_h256_for_ink() {
    let binding = H256::from_slice(&[
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 4, 2,
        6, 9,
    ]);
    let binding = binding.as_fixed_bytes();
    println!("H256 de 'abc': {:?}", binding);
}
