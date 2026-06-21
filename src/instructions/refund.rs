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

struct RefundAccounts<'a> {
    maker: &'a mut AccountView,
    escrow: &'a mut AccountView,
    mint_a: &'a AccountView,
    vault: &'a AccountView,
    maker_ata_a: &'a AccountView,
    system_program: &'a AccountView,
    token_program: &'a AccountView,
}
impl<'a> TryFrom<&'a mut [AccountView]> for RefundAccounts<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a mut [AccountView]) -> Result<Self, Self::Error> {
        let [maker, escrow, mint_a, vault, maker_ata_a, system_program, token_program, _] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        SignerAccount::check(maker)?;
        ProgramAccount::check(escrow)?;
        MintInterface::check(mint_a)?;
        AssociatedTokenAccount::check(maker_ata_a, maker, mint_a, token_program)?;
        AssociatedTokenAccount::check(vault, escrow, mint_a, token_program)?;

        Ok(Self {
            maker,
            escrow,
            mint_a,
            vault,
            maker_ata_a,
            system_program,
            token_program,
        })
    }
}

pub struct Refund<'a> {
    accounts: RefundAccounts<'a>,
}
impl<'a> TryFrom<&'a mut [AccountView]> for Refund<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a mut [AccountView]) -> Result<Self, Self::Error> {
        let accounts = RefundAccounts::try_from(accounts)?;

        CreateIdempotent {
            funding_account: accounts.maker,
            account: accounts.maker_ata_a,
            wallet: accounts.maker,
            mint: accounts.mint_a,
            system_program: accounts.system_program,
            token_program: accounts.token_program,
        }
        .invoke()?;

        Ok(Refund { accounts })
    }
}
impl<'a> Refund<'a> {
    pub const DISCRIMINATOR: &'a u8 = &2u8;

    pub fn process(&mut self) -> ProgramResult {
        let (escrow_seed, escrow_bump) = {
            let data = self.accounts.escrow.try_borrow()?;
            let escrow = Escrow::load(&data)?;
            (escrow.seed, escrow.bump)
        };
        let escrow_seeds = [
            Seed::from(b"escrow"),
            Seed::from(self.accounts.maker.address().as_ref()),
            Seed::from(&escrow_seed),
            Seed::from(&escrow_bump),
        ];

        let amount = Account::from_account_view(self.accounts.vault)?.amount();

        Transfer::new(
            self.accounts.vault,
            self.accounts.maker_ata_a,
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
