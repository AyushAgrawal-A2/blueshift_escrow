use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    AccountView, ProgramResult,
};
use pinocchio_associated_token_account::instructions::CreateIdempotent;
use pinocchio_token::{
    instructions::{CloseAccount, Transfer},
    state::Account,
};

use crate::{
    instructions::helpers::{AssociatedTokenAccount, MintInterface, ProgramAccount, SignerAccount},
    state::Escrow,
};

struct TakeAccount<'a> {
    taker: &'a AccountView,
    maker: &'a mut AccountView,
    escrow: &'a mut AccountView,
    mint_a: &'a AccountView,
    mint_b: &'a AccountView,
    vault: &'a AccountView,
    taker_ata_a: &'a AccountView,
    taker_ata_b: &'a AccountView,
    maker_ata_b: &'a AccountView,
    system_program: &'a AccountView,
    token_program: &'a AccountView,
}
impl<'a> TryFrom<&'a mut [AccountView]> for TakeAccount<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a mut [AccountView]) -> Result<Self, Self::Error> {
        let [taker, maker, escrow, mint_a, mint_b, vault, taker_ata_a, taker_ata_b, maker_ata_b, system_program, token_program, _] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        SignerAccount::check(taker)?;
        ProgramAccount::check(escrow)?;
        MintInterface::check(mint_a)?;
        MintInterface::check(mint_b)?;
        AssociatedTokenAccount::check(taker_ata_b, taker, mint_b, token_program)?;
        AssociatedTokenAccount::check(vault, escrow, mint_a, token_program)?;

        Ok(Self {
            taker,
            maker,
            escrow,
            mint_a,
            mint_b,
            vault,
            taker_ata_a,
            taker_ata_b,
            maker_ata_b,
            system_program,
            token_program,
        })
    }
}

pub struct Take<'a> {
    accounts: TakeAccount<'a>,
}
impl<'a> TryFrom<&'a mut [AccountView]> for Take<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a mut [AccountView]) -> Result<Self, Self::Error> {
        let accounts = TakeAccount::try_from(accounts)?;

        CreateIdempotent {
            funding_account: accounts.taker,
            account: accounts.taker_ata_a,
            wallet: accounts.taker,
            mint: accounts.mint_a,
            system_program: accounts.system_program,
            token_program: accounts.token_program,
        }
        .invoke()?;

        CreateIdempotent {
            funding_account: accounts.taker,
            account: accounts.maker_ata_b,
            wallet: accounts.maker,
            mint: accounts.mint_b,
            system_program: accounts.system_program,
            token_program: accounts.token_program,
        }
        .invoke()?;

        Ok(Take { accounts })
    }
}
impl<'a> Take<'a> {
    pub const DISCRIMINATOR: &'a u8 = &1u8;

    pub fn process(&mut self) -> ProgramResult {
        let (escrow_seed, escrow_bump, escrow_receive) = {
            let data = self.accounts.escrow.try_borrow()?;
            let escrow = Escrow::load(&data)?;
            (escrow.seed, escrow.bump, escrow.receive())
        };

        let escrow_seeds = [
            Seed::from(b"escrow"),
            Seed::from(self.accounts.maker.address().as_ref()),
            Seed::from(&escrow_seed),
            Seed::from(&escrow_bump),
        ];

        let amount = Account::from_account_view(self.accounts.vault)?.amount();

        Transfer::new(
            self.accounts.taker_ata_b,
            self.accounts.maker_ata_b,
            self.accounts.taker,
            escrow_receive,
        )
        .invoke()?;

        Transfer::new(
            self.accounts.vault,
            self.accounts.taker_ata_a,
            self.accounts.escrow,
            amount,
        )
        .invoke_signed(&[Signer::from(&escrow_seeds)])?;

        CloseAccount::new(
            self.accounts.vault,
            self.accounts.maker,
            self.accounts.escrow,
        )
        .invoke_signed(&[Signer::from(&escrow_seeds)])?;

        ProgramAccount::close(self.accounts.escrow, self.accounts.maker)?;

        Ok(())
    }
}
