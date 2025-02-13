import * as anchor from '@coral-xyz/anchor'
import * as sb from "@switchboard-xyz/on-demand";
import {Program} from '@coral-xyz/anchor'
import {Tokenlottery} from '../target/types/tokenlottery'
import { TOKEN_PROGRAM_ID } from '@coral-xyz/anchor/dist/cjs/utils/token';
import { ConnectionContext } from '@solana/wallet-adapter-react';
import { getAssociatedTokenAddressSync } from '@solana/spl-token';
import { IconContrastFilled } from '@tabler/icons-react';




describe('tokenlottery', () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  const connection = provider.connection;
  const wallet = provider.wallet as anchor.Wallet;
  anchor.setProvider(provider);
  
  const rngKp = anchor.web3.Keypair.generate();

  const program = anchor.workspace.Tokenlottery as Program<Tokenlottery>;
  
  let switchboardProgram: Program;
  
  const TOKEN_METADATA_PROGRAM_ID = new anchor.web3.PublicKey('metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s');
  
  const idBuffer = Buffer.alloc(8); // Assuming id is u64
  idBuffer.writeBigUInt64LE(BigInt(17)); // Convert properly
  
  async function buyTicket(){

    const token_lottery = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from('token_lottery'),
        wallet.payer.publicKey.toBuffer(),
        idBuffer
      ],
      program.programId
    )[0];
  
    console.log(token_lottery.toBase58());

    const buyTicketTx = await program.methods.buyTicket().accounts({
      tokenProgram: TOKEN_PROGRAM_ID,
      tokenLottery: token_lottery
    })
    .instruction();

    const computeIx = anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({
      units: 300000
    });

    const priorityIx = anchor.web3.ComputeBudgetProgram.setComputeUnitPrice({
      microLamports: 1
    })

    const blockhashContext = await connection.getLatestBlockhash();

    const tx = new anchor.web3.Transaction({
      feePayer: wallet.payer.publicKey,
      blockhash: blockhashContext.blockhash,
      lastValidBlockHeight: blockhashContext.lastValidBlockHeight,
    }).add(buyTicketTx)
      .add(computeIx)
      .add(priorityIx);
    
      const sig = await anchor.web3.sendAndConfirmTransaction(
        connection,
        tx,
        [wallet.payer],
        {skipPreflight: true}
      );

      console.log("buy ticket", sig);

  }

  beforeAll(async()=>{
    const switchboardIDL = await anchor.Program.fetchIdl(
      sb.ON_DEMAND_DEVNET_PID,
      {
        connection: new anchor.web3.Connection(
          "https://api.devnet.solana.com"
        )
      }
    );

    switchboardProgram = new anchor.Program(switchboardIDL!, provider);
  });

  it('Is initialized!', async ()=>{

    const slot = await connection.getSlot();
    console.log('Current slot:', slot);

    const token_lottery_id = new anchor.BN(17
      
    );

    const initConfigTx = await program.methods.initializeConfig(
      token_lottery_id,
      new anchor.BN(0),
      new anchor.BN(slot + 20),
      new anchor.BN(10000)
    ).instruction();

    const mint = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from('collection_mint'),
        wallet.payer.publicKey.toBuffer(),
        idBuffer
      ],

      program.programId 
    )[0];


    const metadata = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from('metadata'),
        TOKEN_METADATA_PROGRAM_ID.toBuffer(),
        mint.toBuffer(),
      ],
      TOKEN_METADATA_PROGRAM_ID
    )[0];

    const masterEdition = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from('metadata'),
        TOKEN_METADATA_PROGRAM_ID.toBuffer(),
        mint.toBuffer(),
        Buffer.from('edition'),
      ],
      TOKEN_METADATA_PROGRAM_ID
    )[0];

    const initLotteryTx = await program.methods.initializeLottery(
      token_lottery_id
    ).accounts({
      masterEdition,
      metadata,
      tokenProgram: TOKEN_PROGRAM_ID,
    }).instruction();

    const blockhashContext = await connection.getLatestBlockhash();

    const tx = new anchor.web3.Transaction({
      feePayer: wallet.payer.publicKey,
      blockhash: blockhashContext.blockhash,
      lastValidBlockHeight: blockhashContext.lastValidBlockHeight,
    }).add(initConfigTx)
      .add(initLotteryTx);


    const sig = await anchor.web3.sendAndConfirmTransaction(
      connection,
      tx,
      [wallet.payer],
      {skipPreflight: true}
    )
    
    console.log(sig);
  }, 100000)
  
  it('Is buying tickets!', async()=>{
      await buyTicket();
      await buyTicket();
      await buyTicket();
      await buyTicket();
      await buyTicket();

  })

  it('Is committing and revealing a winner', async()=>{

    const token_lottery = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from('token_lottery'),
        wallet.payer.publicKey.toBuffer(),
        idBuffer
      ],
      program.programId
    )[0];

    // create queue instance
    const queueAccount = await sb.getDefaultDevnetQueue();

    console.log("Queue account", queueAccount);
    try {
      await queueAccount.loadData();
    } catch (err) {
      console.log("Queue account not found");
      process.exit(1);
    }
    // return instruction for this task
    const [randomness, txInstruction] = await sb.Randomness.create(
      switchboardProgram,
      rngKp, // keypair of randomness account 
      queueAccount.pubkey,
    );
    console.log("Create randomness account..");
    console.log("Randomness account", randomness.pubkey.toBase58());
    console.log("rkp account", rngKp.publicKey.toBase58());
    
    // convert to version transaction 
    const createRandomnessTx = await sb.asV0Tx({
      connection: connection,
      ixs: [txInstruction],
      payer: wallet.publicKey,
      signers: [wallet.payer, rngKp],
      computeUnitPrice: 75_000,
      computeUnitLimitMultiple: 1.3,
    });


    const blockhashContext = await connection.getLatestBlockhashAndContext();
    
    const createRandomnessSignature = await connection.sendTransaction(
      createRandomnessTx,
      {skipPreflight: true}
    );
    await connection.confirmTransaction({
      signature: createRandomnessSignature,
      blockhash: blockhashContext.value.blockhash,
      lastValidBlockHeight: blockhashContext.value.lastValidBlockHeight,
    });

    console.log(
      "Transaction Signature for randomness account creation: ",
      createRandomnessSignature
    );
    
  
    // return instruction for this task
    const sbCommitTx = await randomness.commitIx(queueAccount.pubkey);
    
    console.log("Commit randomness account..", sbCommitTx);

    const commitIx = await program.methods.commitAWinner().accounts({
      randomnessAccountData: randomness.pubkey,
      tokenLottery: token_lottery
    }).instruction();

    const commitTx = await sb.asV0Tx({
      connection: connection,
      ixs: [sbCommitTx, commitIx],
      payer: wallet.publicKey,
      signers: [wallet.payer],
      computeUnitPrice: 75_000,
      computeUnitLimitMultiple: 1.3,
    });

    const commitSignature = await connection.sendTransaction(commitTx, {skipPreflight: true });

    await connection.confirmTransaction({
      signature: commitSignature,
      blockhash: blockhashContext.value.blockhash,
      lastValidBlockHeight: blockhashContext.value.lastValidBlockHeight,
    });

    console.log(
      "Transaction Signature for committing a winner: ",
      commitSignature
    );

    const sbRevealIx = await randomness.revealIx();
    const revealIx = await program.methods.chooseAWinner().accounts({
      randomnessAccountData: randomness.pubkey,
      tokenLottery: token_lottery
    })
    .instruction();
    
    const revealTx = await sb.asV0Tx({
      connection: connection,
      ixs: [sbRevealIx, revealIx],
      payer: wallet.publicKey,
      signers: [wallet.payer],
      computeUnitPrice: 75_000,
      computeUnitLimitMultiple: 1.3,
    });

    const revealSignature = await connection.sendTransaction(revealTx, {skipPreflight: true });

    await connection.confirmTransaction({
      signature: revealSignature,
      blockhash: blockhashContext.value.blockhash,
      lastValidBlockHeight: blockhashContext.value.lastValidBlockHeight,
    });
    
    console.log(
      "Transaction Signature for revealing a winner: ",
      revealSignature
    );
    
    
  }, 10000)

  it("Is claiming a prize", async()=>{
     
    const token_lottery = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from('token_lottery'),
        wallet.payer.publicKey.toBuffer(),
        idBuffer
      ],
      program.programId
    )[0];
    
    const lotteryConfig = await program.account.tokenLottery.fetch(token_lottery);
    console.log("Lottery winner", lotteryConfig.winner.toNumber());
    console.log("Lottery config", lotteryConfig);

    const tokenAccounts = await connection.getParsedTokenAccountsByOwner(
      wallet.publicKey,
      {
        programId: TOKEN_PROGRAM_ID
      }
    );

    tokenAccounts.value.forEach(async (account)=>{
      console.log("Token account mint", account.account.data.parsed.info.mint);
      console.log("Token account address", account.pubkey.toBase58());
    })

    const winningMint = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from('ticket_mint'),
        wallet.publicKey.toBuffer(),
        new anchor.BN(17).toArrayLike(Buffer),
        lotteryConfig.winner.toArrayLike(Buffer),
      ],
      TOKEN_PROGRAM_ID

      
    )[0];

    console.log("Winning mint", winningMint.toBase58());

    const winningTokenAddress = getAssociatedTokenAddressSync(
      winningMint,
      wallet.publicKey
    );

    console.log("Winning token address", winningTokenAddress.toBase58());
    

   
    const claimIx = await program.methods.claimPrize().accounts({
      tokenProgram: TOKEN_PROGRAM_ID,
      tokenLottery: token_lottery,
      ticketMint: winningMint,
    }).instruction();

    const blockhashContext = await connection.getLatestBlockhashAndContext();

    const claimTx = new anchor.web3.Transaction({
      blockhash: blockhashContext.value.blockhash,
      feePayer: wallet.payer.publicKey,
      lastValidBlockHeight: blockhashContext.value.lastValidBlockHeight
    }).add(claimIx);

    const claimSignature = await anchor.web3.sendAndConfirmTransaction(
      connection,
      claimTx,
      [wallet.payer],
      {skipPreflight: true}
    );

    console.log("Claim signature", claimSignature);
  })
  
})
