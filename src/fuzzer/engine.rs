use std::sync::Mutex;

use contract_transcode::ContractMessageTranscoder;
use frame_support::traits::{OnFinalize, OnInitialize};
use prettytable::{row, Table};

use crate::{
    contract::remote::FullContractResponse,
    contract::runtime::{
        AllPalletsWithSystem, BlockNumber, RuntimeOrigin, Timestamp, SLOT_DURATION,
    },
    fuzzer::bug::BugManager,
    fuzzer::fuzz::Fuzzer,
    fuzzer::parser::OneInput,
};

pub trait FuzzerEngine {
    fn fuzz(self);
    fn harness(
        client: Fuzzer,
        transcoder_loader: &mut Mutex<ContractMessageTranscoder>,
        bug_manager: &mut BugManager,
        input: &[u8],
        save_harness: bool,
    );

    /// Pretty print the result of `OneInput`
    fn pretty_print(responses: Vec<FullContractResponse>, one_input: OneInput) {
        println!("\n🌱 Executing new seed\n");
        let mut table = Table::new();
        table.add_row(row!["Message", "Consummed gas"]);

        for i in 0..responses.len() {
            let curr_result = responses.get(i);

            let description = one_input
                .messages
                .get(i)
                .map(|msg| msg.message_metadata.to_string())
                .unwrap_or_else(|| "FAIL".to_string());

            let debug = match curr_result {
                Some(result) => &result.gas_consumed.to_string(),
                None => &"FAIL".to_string(),
            };

            table.add_row(row![description, debug]);
        }

        table.printstd();
    }

    /// We need to instantiate a proper timestamp on each call
    fn timestamp(lapse: u32) {
        let mut block: u32 = 1;
        Timestamp::set(
            RuntimeOrigin::none(),
            (block as u64).saturating_mul(SLOT_DURATION),
        )
        .unwrap();
        if lapse > 0 {
            <AllPalletsWithSystem as OnFinalize<BlockNumber>>::on_finalize(block);
            block = block.saturating_add(lapse);
            <AllPalletsWithSystem as OnInitialize<BlockNumber>>::on_initialize(block);
            Timestamp::set(
                RuntimeOrigin::none(),
                SLOT_DURATION.saturating_mul(block as u64),
            )
            .unwrap();
        }
    }
    fn exec_seed(self, seed: &[u8]);
}
