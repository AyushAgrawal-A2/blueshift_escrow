use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    AccountView, Address, ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;

use crate::state::Escrow;

pub struct SignerAccount;
impl SignerAccount {
    pub fn check(account: &AccountView) -> ProgramResult {
        if !account.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        Ok(())
    }
}

pub struct MintInterface;
impl MintInterface {
    pub fn check(account: &AccountView) -> ProgramResult {
        if account.owned_by(&pinocchio_token::ID) {
            if account.data_len().ne(&pinocchio_token::state::Mint::LEN) {
                return Err(ProgramError::InvalidAccountData);
            }
            Ok(())
        } else if account.owned_by(&pinocchio_token_2022::ID) {
            if account
                .data_len()
                .lt(&pinocchio_token_2022::state::Mint::BASE_LEN)
            {
                return Err(ProgramError::InvalidAccountData);
            }
            Ok(())
        } else {
            Err(ProgramError::InvalidAccountOwner)
        }
    }
}

pub struct AssociatedTokenAccount;
impl AssociatedTokenAccount {
    pub fn check(
        account: &AccountView,
        authority: &AccountView,
        mint: &AccountView,
        token_program: &AccountView,
    ) -> ProgramResult {
        if Address::derive_program_address(
            &[
                authority.address().as_ref(),
                token_program.address().as_ref(),
                mint.address().as_ref(),
            ],
            &pinocchio_associated_token_account::ID,
        )
        .ok_or(ProgramError::InvalidSeeds)?
        .0
        .ne(account.address())
        {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
}

pub struct ProgramAccount;
impl ProgramAccount {
    pub fn check(account: &AccountView) -> ProgramResult {
        if !account.owned_by(&crate::ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }
        if account.data_len().ne(&Escrow::LEN) {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    pub fn init<'a>(
        payer: &AccountView,
        account: &AccountView,
        seeds: &[Seed<'a>],
        space: usize,
    ) -> ProgramResult {
        let lamports = Rent::get()?.try_minimum_balance(space)?;
        let signer = [Signer::from(seeds)];
        CreateAccount {
            from: payer,
            to: account,
            lamports,
            space: space as u64,
            owner: &crate::ID,
        }
        .invoke_signed(&signer)?;
        Ok(())
    }

    pub fn close(account: &mut AccountView, destination: &mut AccountView) -> ProgramResult {
        let destination_lamports = destination
            .lamports()
            .checked_add(account.lamports())
            .ok_or(ProgramError::ArithmeticOverflow)?;
        destination.set_lamports(destination_lamports);
        account.close()?;
        Ok(())
    }
}
