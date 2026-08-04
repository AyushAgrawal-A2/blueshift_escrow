use pinocchio::{cpi::Seed, error::ProgramError, AccountView, Address, ProgramResult};
use pinocchio_associated_token_account::instructions::Create;
use pinocchio_token::instructions::Transfer;

use crate::{
    instructions::helpers::{AssociatedTokenAccount, MintInterface, ProgramAccount, SignerAccount},
    state::Escrow,
};

struct MakeAccounts<'a> {
    maker: &'a AccountView,
    escrow: &'a mut AccountView,
    mint_a: &'a AccountView,
    mint_b: &'a AccountView,
    maker_ata_a: &'a AccountView,
    vault: &'a AccountView,
    system_program: &'a AccountView,
    token_program: &'a AccountView,
}
impl<'a> TryFrom<&'a mut [AccountView]> for MakeAccounts<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a mut [AccountView]) -> Result<Self, Self::Error> {
        let [maker, escrow, mint_a, mint_b, maker_ata_a, vault, system_program, token_program, _] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        SignerAccount::check(maker)?;
        MintInterface::check(mint_a)?;
        MintInterface::check(mint_b)?;
        AssociatedTokenAccount::check(maker_ata_a, maker, mint_a, token_program)?;

        Ok(Self {
            maker,
            escrow,
            mint_a,
            mint_b,
            maker_ata_a,
            vault,
            system_program,
            token_program,
        })
    }
}

struct MakeInstructionData {
    seed: u64,
    receive: u64,
    amount: u64,
}
impl<'a> TryFrom<&'a [u8]> for MakeInstructionData {
    type Error = ProgramError;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len().ne(&size_of::<MakeInstructionData>()) {
            return Err(ProgramError::InvalidInstructionData);
        }

        let seed = u64::from_le_bytes(data[0..8].try_into().unwrap());
        let receive = u64::from_le_bytes(data[8..16].try_into().unwrap());
        let amount = u64::from_le_bytes(data[16..24].try_into().unwrap());
        if amount == 0 || receive == 0 {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(Self {
            seed,
            receive,
            amount,
        })
    }
}

pub struct Make<'a> {
    accounts: MakeAccounts<'a>,
    instruction_data: MakeInstructionData,
    escrow_bump: [u8; 1],
}
impl<'a> TryFrom<(&'a [u8], &'a mut [AccountView])> for Make<'a> {
    type Error = ProgramError;
    fn try_from((data, accounts): (&'a [u8], &'a mut [AccountView])) -> Result<Self, Self::Error> {
        let accounts = MakeAccounts::try_from(accounts)?;
        let instruction_data = MakeInstructionData::try_from(data)?;

        let seed = instruction_data.seed.to_le_bytes();
        let (escrow, escrow_bump) = Address::derive_program_address(
            &[b"escrow", accounts.maker.address().as_ref(), &seed],
            &crate::ID,
        )
        .ok_or(ProgramError::InvalidSeeds)?;
        if accounts.escrow.address().ne(&escrow) {
            return Err(ProgramError::InvalidArgument);
        }
        let escrow_bump = [escrow_bump];
        let escrow_seeds = [
            Seed::from(b"escrow"),
            Seed::from(accounts.maker.address().as_ref()),
            Seed::from(&seed),
            Seed::from(&escrow_bump),
        ];

        ProgramAccount::init(
            accounts.maker,
            accounts.escrow,
            &escrow_seeds[..],
            Escrow::LEN,
        )?;

        Create {
            funding_account: accounts.maker,
            account: accounts.vault,
            wallet: accounts.escrow,
            mint: accounts.mint_a,
            system_program: accounts.system_program,
            token_program: accounts.token_program,
        }
        .invoke()?;

        Ok(Make {
            accounts,
            instruction_data,
            escrow_bump,
        })
    }
}
impl<'a> Make<'a> {
    pub const DISCRIMINATOR: &'a u8 = &0u8;

    pub fn process(&mut self) -> ProgramResult {
        {
            let mut data = self.accounts.escrow.try_borrow_mut()?;
            let escrow = Escrow::load_mut(&mut data)?;
            escrow.set_inner(
                self.instruction_data.seed,
                *self.accounts.maker.address(),
                *self.accounts.mint_a.address(),
                *self.accounts.mint_b.address(),
                self.instruction_data.receive,
                self.escrow_bump,
            );
        }

        Transfer::new(
            self.accounts.maker_ata_a,
            self.accounts.vault,
            self.accounts.maker,
            self.instruction_data.amount,
        )
        .invoke()?;

        Ok(())
    }
}
