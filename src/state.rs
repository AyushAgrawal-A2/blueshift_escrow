use pinocchio::{error::ProgramError, Address};

#[repr(C)]
pub struct Escrow {
    pub seed: [u8; 8], // u64
    pub maker: Address,
    pub mint_a: Address,
    pub mint_b: Address,
    pub receive: [u8; 8], // u64
    pub bump: [u8; 1],
}

impl Escrow {
    pub const LEN: usize = size_of::<Escrow>();

    #[inline(always)]
    pub fn load_mut(bytes: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(unsafe { &mut *bytes.as_mut_ptr().cast::<Self>() })
    }

    #[inline(always)]
    pub fn load(bytes: &[u8]) -> Result<&Self, ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &*core::mem::transmute::<*const u8, *const Self>(bytes.as_ptr()) })
    }

    #[inline(always)]
    pub fn receive(&self) -> u64 {
        u64::from_le_bytes(self.receive)
    }

    #[inline(always)]
    pub fn set_inner(
        &mut self,
        seed: u64,
        maker: Address,
        mint_a: Address,
        mint_b: Address,
        receive: u64,
        bump: [u8; 1],
    ) {
        self.seed = seed.to_le_bytes();
        self.maker = maker;
        self.mint_a = mint_a;
        self.mint_b = mint_b;
        self.receive = receive.to_le_bytes();
        self.bump = bump;
    }
}
