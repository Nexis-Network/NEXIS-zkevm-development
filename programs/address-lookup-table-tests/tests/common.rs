#![allow(dead_code)]
use {
    nexis_address_lookup_table_program::{
        id,
        processor::process_instruction,
        state::{AddressLookupTable, LookupTableMeta},
    },
    nexis_program_test::*,
    nexis_sdk::{
        account::AccountSharedData,
        clock::Slot,
        hash::Hash,
        instruction::Instruction,
        instruction::InstructionError,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        slot_hashes::SlotHashes,
        transaction::{Transaction, TransactionError},
    },
    std::borrow::Cow,
};

pub async fn setup_test_context() -> ProgramTestContext {
    let program_test = ProgramTest::new("", id(), Some(process_instruction));
    program_test.start_with_context().await
}

pub async fn assert_ix_error(
    context: &mut ProgramTestContext,
    ix: Instruction,
    authority_keypair: Option<&Keypair>,
    expected_err: InstructionError,
) {
    let client = &mut context.banks_client;
    let payer = &context.payer;
    let recent_blockhash = context.last_blockhash;

    let mut signers = vec![payer];
    if let Some(authority) = authority_keypair {
        signers.push(authority);
    }

    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &signers,
        recent_blockhash,
    );

    assert_eq!(
        client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, expected_err),
    );
}

pub fn new_address_lookup_table(
    authority: Option<Pubkey>,
    num_addresses: usize,
) -> AddressLookupTable<'static> {
    let mut addresses = Vec::with_capacity(num_addresses);
    addresses.resize_with(num_addresses, Pubkey::new_unique);
    AddressLookupTable {
        meta: LookupTableMeta {
            authority,
            ..LookupTableMeta::default()
        },
        addresses: Cow::Owned(addresses),
    }
}

pub async fn add_lookup_table_account(
    context: &mut ProgramTestContext,
    account_address: Pubkey,
    address_lookup_table: AddressLookupTable<'static>,
) -> AccountSharedData {
    let mut data = Vec::new();
    address_lookup_table.serialize_for_tests(&mut data).unwrap();

    let rent = context.banks_client.get_rent().await.unwrap();
    let rent_exempt_balance = rent.minimum_balance(data.len());

    let mut account = AccountSharedData::new(
        rent_exempt_balance,
        data.len(),
        &nexis_address_lookup_table_program::id(),
    );
    account.set_data(data);
    context.set_account(&account_address, &account);

    account
}

pub fn overwrite_slot_hashes_with_slots(context: &mut ProgramTestContext, slots: &[Slot]) {
    let mut slot_hashes = SlotHashes::default();
    for slot in slots {
        slot_hashes.add(*slot, Hash::new_unique());
    }
    context.set_sysvar(&slot_hashes);
}