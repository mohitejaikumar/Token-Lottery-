use anchor_lang::prelude::*;
use anchor_lang::system_program;
use switchboard_on_demand::accounts::RandomnessAccountData;
use anchor_spl::{
  metadata::{
    Metadata,
    MetadataAccount,
    CreateMetadataAccountsV3,
    CreateMasterEditionV3,
    SignMetadata,
    SetAndVerifySizedCollectionItem,
    create_metadata_accounts_v3,
    create_master_edition_v3,
    sign_metadata,
    set_and_verify_sized_collection_item,
    mpl_token_metadata::types::{
      DataV2,
      Creator,
      CollectionDetails
    },
  }, 
  token_interface::{
    Mint, 
    mint_to,
    MintTo,
    TokenInterface, 
    TokenAccount
  },
  associated_token::AssociatedToken
};

declare_id!("CWDQ2VCFe9GLbSxS92wQ23wrB8h176B98NVgrDJJdSLW");


#[constant]
pub const NAME: &str = "Token Lottery Ticket";

#[constant]
pub const URI: &str = "Token Lottery";

#[constant]
pub const SYMBOL: &str = "TICKET";

#[program]
pub mod tokenlottery {

    use super::*;

    pub fn initialize_config(
      ctx: Context<InitializeConfig>,
      id: u64,
      start: u64,
      end: u64,
      price: u64,
    ) -> Result<()> {
      ctx.accounts.token_lottery.id = id;
      ctx.accounts.token_lottery.bump = ctx.bumps.token_lottery;
      ctx.accounts.token_lottery.lottery_start = start;
      ctx.accounts.token_lottery.lottery_end = end;
      ctx.accounts.token_lottery.price = price;
      ctx.accounts.token_lottery.number_of_tickets = 0;
      ctx.accounts.token_lottery.lottery_pot_amount = 0;
      ctx.accounts.token_lottery.authority = ctx.accounts.payer.key();
      ctx.accounts.token_lottery.randomness_account = Pubkey::default();
      ctx.accounts.token_lottery.is_winner_chosen = false;
      
      Ok(())
    }

    pub fn initialize_lottery(
       ctx: Context<InitializeLottery>,
       id: u64
    ) -> Result<()> {
      let pkey = ctx.accounts.payer.key();
      let signer_seeds: &[&[&[u8]]] = &[&[
        b"collection_mint".as_ref(),
        pkey.as_ref(),
        &id.to_le_bytes(),
        &[ctx.bumps.collection_mint]
      ]];

      // mint the nft 
      // create metadata
      // create master edition
      // sing_metadata
      msg!("Collection mint : {:?}", ctx.accounts.collection_mint.key());
      msg!("Collection mint seeds : {:?}", signer_seeds);
      msg!("Collection mint bump : {:?}", ctx.bumps.collection_mint);
      msg!("Mint the collection NFT");
      let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        MintTo {
          mint: ctx.accounts.collection_mint.to_account_info(),
          to: ctx.accounts.collection_token_account.to_account_info(),
          authority: ctx.accounts.collection_mint.to_account_info()
        },
        signer_seeds
      );

      mint_to(
        cpi_context,
        1
      )?;
      
      msg!("Creating Metadata accounts");

      let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.token_metadata_program.to_account_info(),
        CreateMetadataAccountsV3 {
          metadata: ctx.accounts.metadata.to_account_info(),
          mint: ctx.accounts.collection_mint.to_account_info(),
          mint_authority: ctx.accounts.collection_mint.to_account_info(),
          payer: ctx.accounts.payer.to_account_info(),
          update_authority: ctx.accounts.collection_mint.to_account_info(),
          system_program: ctx.accounts.system_program.to_account_info(),
          rent: ctx.accounts.rent.to_account_info()
        },
        signer_seeds
      );
      create_metadata_accounts_v3(
        cpi_context,
        DataV2 {
          name: NAME.to_string(),
          symbol: SYMBOL.to_string(),
          uri: URI.to_string(),
          seller_fee_basis_points: 0,
          creators: Some(vec![Creator {
            address: ctx.accounts.collection_mint.key(),
            verified: false,
            share: 100
          }]),
          collection: None,
          uses: None,
        },
        true,
        true,
        Some(CollectionDetails::V1 { size: 0 }),
      )?;

      msg!("Creating Master Edition accounts");

      let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.token_metadata_program.to_account_info(),
        CreateMasterEditionV3 {
          payer: ctx.accounts.payer.to_account_info(),
          edition: ctx.accounts.master_edition.to_account_info(),
          mint: ctx.accounts.collection_mint.to_account_info(),
          update_authority: ctx.accounts.collection_mint.to_account_info(),
          mint_authority: ctx.accounts.collection_mint.to_account_info(),
          metadata: ctx.accounts.metadata.to_account_info(),
          token_program: ctx.accounts.token_program.to_account_info(),
          system_program: ctx.accounts.system_program.to_account_info(),
          rent: ctx.accounts.rent.to_account_info()
        },
        signer_seeds
      );

      create_master_edition_v3(
        cpi_context,
        Some(0)
      )?;
      

      msg!("verifying collection");

      let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.token_metadata_program.to_account_info(),
        SignMetadata {
          creator: ctx.accounts.collection_mint.to_account_info(),
          metadata: ctx.accounts.metadata.to_account_info(),
        },
        signer_seeds
      );

      sign_metadata(
        cpi_context
      )?;

      Ok(())
    }
    
     pub fn buy_ticket(
      ctx: Context<BuyTicket>,
     ) -> Result<()> {
      let clock = Clock::get()?;

      // check is lottery is still open
      if clock.slot < ctx.accounts.token_lottery.lottery_start || clock.slot > ctx.accounts.token_lottery.lottery_end {
         return Err(ErrorCode::LotteryNotOpen.into());
      }

      let ticket_name = NAME.to_owned() + ctx.accounts.token_lottery.number_of_tickets.to_string().as_str();

      // transfer solana
      system_program::transfer(
        CpiContext::new(
          ctx.accounts.system_program.to_account_info(),
          system_program::Transfer {
            from: ctx.accounts.payer.to_account_info(),
            to: ctx.accounts.token_lottery.to_account_info(),
          }
        ),
        ctx.accounts.token_lottery.price,
      )?;

      ctx.accounts.token_lottery.lottery_pot_amount += ctx.accounts.token_lottery.price;

      let pkey = ctx.accounts.payer.key();
      let signer_seeds: &[&[&[u8]]] = &[&[
        b"collection_mint".as_ref(),
        pkey.as_ref(),
        &ctx.accounts.token_lottery.id.to_le_bytes(),
        &[ctx.bumps.collection_mint]
      ]];

      msg!("Number of tickets: {}, token lottery id {}", ctx.accounts.token_lottery.number_of_tickets, ctx.accounts.token_lottery.id);

      // mint the ticket
      msg!("Mint the collection NFT");
      let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        MintTo {
          mint: ctx.accounts.ticket_mint.to_account_info(),
          to: ctx.accounts.destination_token_account.to_account_info(),
          authority: ctx.accounts.collection_mint.to_account_info()
        },
        signer_seeds
      );

      mint_to(
        cpi_context,
        1
      )?;

      msg!("Creating Metadata accounts");

      let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.token_metadata_program.to_account_info(),
        CreateMetadataAccountsV3 {
          metadata: ctx.accounts.metadata.to_account_info(),
          mint: ctx.accounts.ticket_mint.to_account_info(),
          mint_authority: ctx.accounts.collection_mint.to_account_info(),
          payer: ctx.accounts.payer.to_account_info(),
          update_authority: ctx.accounts.collection_mint.to_account_info(),
          system_program: ctx.accounts.system_program.to_account_info(),
          rent: ctx.accounts.rent.to_account_info()
        },
        signer_seeds
      );
      create_metadata_accounts_v3(
        cpi_context,
        DataV2 {
          name: ticket_name,
          symbol: SYMBOL.to_string(),
          uri: URI.to_string(),
          seller_fee_basis_points: 0,
          creators: Some(vec![Creator {
            address: ctx.accounts.collection_mint.key(),
            verified: false,
            share: 100
          }]),
          collection: None,
          uses: None,
        },
        true,
        true,
        None,
      )?;

      msg!("Creating Master Edition accounts");

      let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.token_metadata_program.to_account_info(),
        CreateMasterEditionV3 {
          payer: ctx.accounts.payer.to_account_info(),
          edition: ctx.accounts.master_edition.to_account_info(),
          mint: ctx.accounts.ticket_mint.to_account_info(),
          update_authority: ctx.accounts.collection_mint.to_account_info(),
          mint_authority: ctx.accounts.collection_mint.to_account_info(),
          metadata: ctx.accounts.metadata.to_account_info(),
          token_program: ctx.accounts.token_program.to_account_info(),
          system_program: ctx.accounts.system_program.to_account_info(),
          rent: ctx.accounts.rent.to_account_info()
        },
        signer_seeds
      );

      create_master_edition_v3(
        cpi_context,
        Some(0)
      )?;
      

      msg!("verifying");

      let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.token_metadata_program.to_account_info(),
        SetAndVerifySizedCollectionItem {
          metadata: ctx.accounts.metadata.to_account_info(),
          payer: ctx.accounts.payer.to_account_info(),
          collection_authority: ctx.accounts.collection_mint.to_account_info(),
          collection_master_edition: ctx.accounts.collection_master_edition.to_account_info(),
          collection_metadata: ctx.accounts.collection_metadata.to_account_info(),
          collection_mint: ctx.accounts.collection_mint.to_account_info(),
          update_authority: ctx.accounts.collection_mint.to_account_info(),
        },
        signer_seeds
      );

      set_and_verify_sized_collection_item(
        cpi_context,
        None
      )?;

      ctx.accounts.token_lottery.number_of_tickets += 1;

      Ok(())
     }
     
     pub fn commit_a_winner(
      ctx: Context<CommitWinner>,
     ) -> Result<()> {
      
      let clock = Clock::get()?;
      let token_lottery = &mut ctx.accounts.token_lottery;
      if ctx.accounts.payer.key() != token_lottery.authority {
        return Err(ErrorCode::NotAuthorized.into());
      }
      
      let randomness_data = RandomnessAccountData::parse(
        ctx.accounts.randomness_account_data.data.borrow()
      ).unwrap();

      if randomness_data.seed_slot != clock.slot - 1 {
        return Err(ErrorCode::RandomnessAlreadyRevealed.into());
      }
      
      token_lottery.randomness_account = ctx.accounts.randomness_account_data.key();

      Ok(())
     }

     pub fn choose_a_winner(
      ctx: Context<ChooseWinner>,
     ) -> Result<()> {
      
      let clock = Clock::get()?;
      let token_lottery = &mut ctx.accounts.token_lottery;

      if ctx.accounts.randomness_account_data.key() != token_lottery.randomness_account {
        return Err(ErrorCode::IncorrectRandomnessAccount.into());
      }

      if ctx.accounts.payer.key() != token_lottery.authority {
        return Err(ErrorCode::NotAuthorized.into());
      }

      if clock.slot < token_lottery.lottery_end {
        msg!("Current Slot: {}", clock.slot);
        msg!("Lottery End Slot: {}", token_lottery.lottery_end);
        return Err(ErrorCode::LotteryNotOpen.into());
      }

      require!(token_lottery.is_winner_chosen == false, ErrorCode::WinnerChosen);
      
      let randomness_data = RandomnessAccountData::parse(
        ctx.accounts.randomness_account_data.data.borrow()
      ).unwrap();

      let revealed_random_value = randomness_data.get_value(&clock).map_err(|_| ErrorCode::RandomnessNotResolved)?;

      msg!("Random Value: {}", revealed_random_value[0]);
      msg!("Number of Tickets: {}", token_lottery.number_of_tickets);

      let randomness_results = revealed_random_value[0] as u64 % token_lottery.number_of_tickets;

      msg!("Winner: {}", randomness_results);

      token_lottery.winner = randomness_results;
      token_lottery.is_winner_chosen = true;                                         
      Ok(())
     }
     
     pub fn claim_prize(
       ctx: Context<ClaimPrize>,
     ) -> Result<()> {

      msg!("Winner chosen: {}", ctx.accounts.token_lottery.is_winner_chosen);
      require!(ctx.accounts.token_lottery.is_winner_chosen, ErrorCode::WinnerNotChosen);
      
      // Check if token is a part of the collection
      require!(ctx.accounts.metadata.collection.as_ref().unwrap().verified, ErrorCode::NotVerifiedTicket);
      require!(ctx.accounts.metadata.collection.as_ref().unwrap().key == ctx.accounts.collection_mint.key(), ErrorCode::IncorrectTicket);
      
      let ticket_name = NAME.to_owned() + &ctx.accounts.token_lottery.winner.to_string();
      let metadata_name = ctx.accounts.metadata.name.replace("\u{0}", "");
      
      msg!("Ticket name: {}", ticket_name);
      msg!("Metdata name: {}", metadata_name);

      // Check if the winner has the winning ticket
      require!(metadata_name == ticket_name, ErrorCode::IncorrectTicket);
      require!(ctx.accounts.destination_token_account.amount > 0, ErrorCode::IncorrectTicket);
     
      **ctx.accounts.token_lottery.to_account_info().try_borrow_mut_lamports()? -= ctx.accounts.token_lottery.lottery_pot_amount;
      **ctx.accounts.payer.try_borrow_mut_lamports()? += ctx.accounts.token_lottery.lottery_pot_amount;
      
      ctx.accounts.token_lottery.lottery_pot_amount = 0;

      Ok(())
     
     }


}



#[derive(Accounts)]
#[instruction(id: u64)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
      init,
      payer = payer,
      space = 8 + TokenLottery::INIT_SPACE,
      seeds = [
        b"token_lottery".as_ref(),
        payer.key().as_ref(), 
        id.to_le_bytes().as_ref(),
      ],
      bump
    )]
    pub token_lottery: Account<'info, TokenLottery>,

    pub system_program: Program<'info, System>

}

#[derive(Accounts)]
#[instruction(id: u64)]
pub struct InitializeLottery<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    
    #[account(
      init,
      payer = payer,
      mint::authority = collection_mint,
      mint::decimals = 0,
      mint::freeze_authority = collection_mint,
      seeds = [
        b"collection_mint".as_ref(),
        payer.key().as_ref(),
        id.to_le_bytes().as_ref(),
      ],
      bump,
    )]
    pub collection_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    /// CHECK: This account will be initialized by the metaplex program
    pub metadata: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: This account will be initialized by the metaplex program
    pub master_edition: UncheckedAccount<'info>,

    #[account(
      init_if_needed,
      payer = payer,
      token::mint = collection_mint,
      token::authority = collection_mint,
      seeds = [
        b"collection_token_account".as_ref(),
        payer.key().as_ref(),
        id.to_le_bytes().as_ref(),
      ],
      bump
    )]
    pub collection_token_account: Box<InterfaceAccount<'info, TokenAccount>>,


    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub rent: Sysvar<'info, Rent>,
}


#[derive(Accounts)]
pub struct BuyTicket<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub token_lottery: Account<'info, TokenLottery>,

    #[account(
      init,
      payer = payer,
      seeds = [
        b"ticket_mint".as_ref(),
        payer.key().as_ref(),
        token_lottery.id.to_le_bytes().as_ref(),
        token_lottery.number_of_tickets.to_le_bytes().as_ref()
      ],
      bump,
      mint::authority = collection_mint,
      mint::decimals = 0,
      mint::freeze_authority = collection_mint,
      mint::token_program = token_program
    )]
    pub ticket_mint: InterfaceAccount<'info, Mint>,

    #[account(
      init,
      payer = payer,
      associated_token::mint = ticket_mint,
      associated_token::authority = payer,
      associated_token::token_program = token_program,
    )]
    pub destination_token_account: InterfaceAccount<'info, TokenAccount>,
    

    #[account(
      mut,
      seeds = [
        b"metadata",
        token_metadata_program.key().as_ref(),
        ticket_mint.key().as_ref()
      ],
      bump,
      seeds::program = token_metadata_program.key(),
    )]
    /// CHECK: This account will be initialized by the metaplex program
    pub metadata: UncheckedAccount<'info>,

    #[account(
      mut,
      seeds = [
        b"metadata",
        token_metadata_program.key().as_ref(),
        ticket_mint.key().as_ref(),
        b"edition"
      ],
      bump,
      seeds::program = token_metadata_program.key(),
    )]
    /// CHECK: This account will be initialized by the metaplex program
    pub master_edition: UncheckedAccount<'info>,

    #[account(
      mut,
      seeds = [
        b"metadata",
        token_metadata_program.key().as_ref(),
        collection_mint.key().as_ref()
      ],
      bump,
      seeds::program = token_metadata_program.key()
    )]
    /// CHECK: This account will be initialized by the metaplex program
    pub collection_metadata: UncheckedAccount<'info>,

    #[account(
      mut,
      seeds = [
        b"metadata",
        token_metadata_program.key().as_ref(),
        collection_mint.key().as_ref(),
        b"edition"
      ],
      bump,
      seeds::program = token_metadata_program.key(),
    )]
    /// CHECK: This account will be initialized by the metaplex program
    pub collection_master_edition: UncheckedAccount<'info>,


    #[account(
      mut,
      seeds = [
        b"collection_mint",
        payer.key().as_ref(),
        token_lottery.id.to_le_bytes().as_ref(),
      ],
      bump,
    )]
    pub collection_mint: InterfaceAccount<'info, Mint>,


    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct CommitWinner<'info> {
  #[account(mut)]
  pub payer: Signer<'info>,

  #[account(mut)]
  pub token_lottery: Account<'info, TokenLottery>,
  /// CHECK: This account will be initialized by the metaplex program
  pub randomness_account_data: UncheckedAccount<'info>,
  
  pub system_program: Program<'info, System>,
}


#[derive(Accounts)]
pub struct ChooseWinner<'info> {
  #[account(mut)]
  pub payer: Signer<'info>,

  #[account(mut)]
  pub token_lottery: Account<'info, TokenLottery>,
  /// CHECK: This account will be initialized by the metaplex program
  pub randomness_account_data: UncheckedAccount<'info>,
  
  pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimPrize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub token_lottery: Account<'info, TokenLottery>,

    #[account(mut)]
    pub ticket_mint: InterfaceAccount<'info, Mint>,

    #[account(
      associated_token::mint = ticket_mint,
      associated_token::authority = payer,
      associated_token::token_program = token_program,
    )]
    pub destination_token_account: InterfaceAccount<'info, TokenAccount>,
    

    #[account(
      seeds = [
        b"metadata".as_ref(),
        token_metadata_program.key().as_ref(),
        ticket_mint.key().as_ref()
      ],
      bump,
      seeds::program = token_metadata_program.key(),
    )]
    pub metadata: Account<'info, MetadataAccount>,

    #[account(
      mut,
      seeds = [
        b"metadata".as_ref(),
        token_metadata_program.key().as_ref(),
        collection_mint.key().as_ref()
      ],
      bump,
      seeds::program = token_metadata_program.key()
    )]
    pub collection_metadata: Account<'info, MetadataAccount>,


    #[account(
      mut,
      seeds = [
        b"collection_mint".as_ref(),
        payer.key().as_ref(),
        token_lottery.id.to_le_bytes().as_ref(),
      ],
      bump,
    )]
    pub collection_mint: InterfaceAccount<'info, Mint>,


    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub token_metadata_program: Program<'info, Metadata>,
    
}

#[error_code]
pub enum ErrorCode{
  #[msg("Lottery is not open")]
  LotteryNotOpen,
  #[msg("Not authorized")]
  NotAuthorized,
  #[msg("Randomness already revealed")]
  RandomnessAlreadyRevealed,
  #[msg("Incorrect randomness account")]
  IncorrectRandomnessAccount,
  #[msg("Winner already chosen")]
  WinnerChosen,
  #[msg("Randomness not resolved")]
  RandomnessNotResolved,
  #[msg("Winner not chosen")]
  WinnerNotChosen,
  #[msg("Not verified ticket")]
  NotVerifiedTicket,
  #[msg("Incorrect ticket")]
  IncorrectTicket,


}



#[account]
#[derive(InitSpace)]
pub struct TokenLottery{
    pub id: u64,
    pub bump: u8,
    pub winner: u64,
    pub is_winner_chosen: bool,
    pub lottery_start: u64,
    pub lottery_end: u64,
    pub price: u64,
    pub number_of_tickets: u64,
    pub lottery_pot_amount: u64,
    pub authority: Pubkey,
    pub randomness_account: Pubkey
}