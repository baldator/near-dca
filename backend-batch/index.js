const nearAPI = require("near-api-js");
const { connect, keyStores } = nearAPI;
const path = require('path');
const fs = require('fs');
const { log } = require("console");
const { Telegraf } = require('telegraf')
const { message } = require('telegraf/filters')
const dotenv = require('dotenv');


dotenv.config();
const NEAR_NET = 'testnet';
const CONTRACT_ID = 'test.dca-near.testnet';
const ACCOUNT_ID = 'dca-near.testnet';
const LOG_FILE_BATCH = `./logs/dca-batch-${new Date().toISOString().split('T')[0]}.log`;
const LOG_FILE_BOT = `./logs/dca-bot-${new Date().toISOString().split('T')[0]}.log`;

// Set up the key store
const keyStore = new keyStores.UnencryptedFileSystemKeyStore(path.join(__dirname, '/near-credentials'));
// Configuration for NEAR for testnet and mainnet
const configTestnet = {
  networkId: 'testnet', // or 'mainnet'
  keyStore: keyStore, // Use browser local storage for keys
  nodeUrl: 'https://rpc.testnet.near.org',
  walletUrl: 'https://wallet.testnet.near.org',
  helperUrl: 'https://helper.testnet.near.org',
  explorerUrl: 'https://explorer.testnet.near.org',
};

const configMainnet = {
  networkId: 'mainnet', // or 'mainnet'
  keyStore: keyStore, // Use browser local storage for keys
  nodeUrl: 'https://rpc.mainnet.near.org',
  walletUrl: 'https://wallet.mainnet.near.org',
  helperUrl: 'https://helper.mainnet.near.org',
  explorerUrl: 'https://explorer.mainnet.near.org',
};

if(NEAR_NET === 'testnet') {
  config = configTestnet;
} else if(NEAR_NET === 'mainnet') {
  config = configMainnet;
}

const logStream = fs.createWriteStream(LOG_FILE_BATCH, { flags: 'a' });
logStream.write(`----------------------------------------------------\n`);
logStream.write(`Start execution: ${new Date().toISOString()}\n`);

async function main() {
  // Connect to NEAR
  const near = await connect(config);
  const accountId = ACCOUNT_ID; // Make sure to replace this with your accountId
  const account = await near.account(accountId);

  // Call a view function
  try {
    
        const response = await account.functionCall({
            contractId: CONTRACT_ID,
            methodName: 'swap',
            args: {
            // Change method arguments go here
            },
            gas: '300000000000000', // Adjust gas accordingly
            attachedDeposit: '1', // Optional: attach NEAR tokens if needed
        });
        logStream.write(`Write function result: ${JSON.stringify(response)}\n`);
        
  } catch (error) {
    logStream.write(`Error calling view function: ${JSON.stringify(error)}\n`);
  }
}

function loop() {
  main().then(() => {
    setTimeout(loop, 5 * 60 * 1000); // 5 minutes
    const nextExecution = new Date(Date.now() + 5 * 60 * 1000);
    logStream.write(`Next execution in 5 minutes... at ${nextExecution.toISOString()}\n`);
    logStream.write(`----------------------------------------------------\n`);
  }).catch((error) => {
    logStream.write(`Error calling main function: ${error}\n`);
    console.error(error);
    setTimeout(loop, 5 * 60 * 1000); // 5 minutes
  });
}

loop();



// BOT CODE
const bot = new Telegraf(process.env.BOT_TOKEN)
bot.start((ctx) => ctx.reply('Welcome'))
bot.help((ctx) => ctx.reply('Send me a sticker'))
bot.on(message('sticker'), (ctx) => ctx.reply('👍'))
bot.hears('hi', (ctx) => ctx.reply('Hey there'))
bot.launch()

// Enable graceful stop
process.once('SIGINT', () => bot.stop('SIGINT'))
process.once('SIGTERM', () => bot.stop('SIGTERM'))