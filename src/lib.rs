#![no_std]

mod instructions;
mod state;

use pinocchio::{entrypoint, nostd_panic_handler, AccountView, Address, ProgramResult};

use crate::instructions::{Make, Refund, Take};

pub const ID: Address = Address::from_str_const("22222222222222222222222222222222222222222222");

entrypoint!(process_instruction);
nostd_panic_handler!();

fn process_instruction(
    _program_id: &Address,
    accounts: &mut [AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    match instruction_data.split_first() {
        Some((Make::DISCRIMINATOR, data)) => Make::try_from((data, accounts))?.process(),
        Some((Take::DISCRIMINATOR, _)) => Take::try_from(accounts)?.process(),
        Some((Refund::DISCRIMINATOR, _)) => Refund::try_from(accounts)?.process(),
        _ => Err(pinocchio::error::ProgramError::InvalidInstructionData),
    }
}
