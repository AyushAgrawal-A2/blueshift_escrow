# blueshift_escrow

Solution to Blueshift's **Pinocchio Escrow** challenge: a maker/taker atomic swap written
`#![no_std]` with every check hand-written. It keeps the challenge's wire format — 1-byte
discriminators (`0` make, `1` take, `2` refund) and little-endian instruction data.

Dependencies: `pinocchio 0.11.2`, `pinocchio-system 0.6.1`, `pinocchio-token 0.6.0`,
`pinocchio-token-2022 0.3.1`, `pinocchio-associated-token-account 0.4.0`. Program id is
`22222222222222222222222222222222222222222222` (Blueshift's fixed challenge address).

Escrow PDA: seeds `[b"escrow", maker, seed.to_le_bytes()]`. Vault: ATA of the escrow PDA for
`mint_a`.

## State

`Escrow`, 113 bytes, `#[repr(C)]`, read zero-copy:

| offset | field | size |
|--------|-------|------|
| 0 | seed (u64 LE) | 8 |
| 8 | maker | 32 |
| 40 | mint_a | 32 |
| 72 | mint_b | 32 |
| 104 | receive (u64 LE) | 8 |
| 112 | bump | 1 |

Every field is a byte array (alignment 1, no padding), so `load`/`load_mut` are an exact-length
check plus a pointer cast. There is no discriminator byte: with a single account type, "owned by
this program and exactly 113 bytes long" *is* the type check (`ProgramAccount::check`). The Anchor
twin spends one extra byte on a discriminator (114 total).

## Instructions

**`make`** — data `seed: u64, receive: u64, amount: u64` (LE); accounts
`[maker, escrow, mint_a, mint_b, maker_ata_a, vault, system_program, token_program, _]`.
Verifies the maker signs, both mints parse, `maker_ata_a` is the maker's ATA for `mint_a`, and
`escrow` matches the derived PDA; then creates the escrow with a PDA-signed `CreateAccount` at the
rent-exempt minimum for 113 bytes, creates the vault ATA, writes the state, and transfers `amount`
into the vault. Rejects `amount == 0` and `receive == 0`.

**`take`** — accounts `[taker, maker, escrow, mint_a, mint_b, vault, taker_ata_a, taker_ata_b,
maker_ata_b, system_program, token_program, _]`. Checks the taker signs, the escrow's owner and
length, that the passed `mint_a` and `mint_b` equal `escrow.mint_a` and `escrow.mint_b`,
`taker_ata_b` = ATA(taker, mint_b), vault = ATA(escrow, mint_a); creates `taker_ata_a` and
`maker_ata_b` with `CreateIdempotent` (the ATA program enforces their derivation on-chain). Then,
atomically: `receive` of `mint_b` taker → maker, full vault balance → taker signed with
`[b"escrow", maker, seed, bump]` from state, close the vault, close the escrow — all rent to the
maker.

**`refund`** — maker signs; the passed `mint_a` must equal `escrow.mint_a`; vault balance back to
`maker_ata_a` (created idempotently), vault and escrow closed to the maker, signed with the same
seeds.

## Validation model

- Mints are validated structurally by owner: SPL Token with exactly `Mint::LEN` data, or
  Token-2022 with at least `Mint::BASE_LEN` — both token programs are accepted.
- On `take` and `refund` the passed mints are compared byte-for-byte to the addresses `make`
  recorded in the escrow (`escrow.mint_a`, and `escrow.mint_b` on take) — the hand-written
  equivalent of Anchor's `has_one` constraints.
- The passed `maker` is bound *implicitly*: the CPI signer seeds include it, so a wrong maker
  derives a PDA that is not the escrow account and every `invoke_signed` fails signature
  verification.
- ATAs are either re-derived with `derive_program_address` against the ATA program or created via
  `CreateIdempotent`, which enforces the same derivation.
- `ProgramAccount::close` moves lamports with `checked_add` before zeroing the account.

## Build / test

```console
$ cargo build-sbf     # → target/deploy/blueshift_escrow.so
```

No tests in the repo; the challenge is graded by Blueshift's own suite against the compiled
program.
