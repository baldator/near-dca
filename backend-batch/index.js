const nearAPI = require("near-api-js");
const { connect, keyStores } = nearAPI;
const path = require('path');
const fs = require('fs');
const { log } = require("console");
const { Telegraf } = require('telegraf')
const { message } = require('telegraf/filters')
const dotenv = require('dotenv');
const { env } = require("process");


dotenv.config();
const NEAR_NET = 'testnet';
const CONTRACT_ID = 'test.dca-near.testnet';
const ACCOUNT_ID = 'dca-near.testnet';
const LOG_FILE_BATCH = `./logs/dca-batch-${new Date().toISOString().split('T')[0]}.log`;
const LOG_FILE_BOT = `./logs/dca-bot-${new Date().toISOString().split('T')[0]}.log`;
const DB_FILE = env.DATABASE_FILE || './data/dca-batch.db';
const TELEGRAM_BOT_TOKEN = env.TELEGRAM_BOT_TOKEN || 'TELEGRAM_BOT_TOKEN';

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

// common functions
// check if address is a valid near address
async function isValidNearAddress(nearConfig, accountId) {
    try {
      const near = await connect(nearConfig);
      const account = await near.account(accountId);
      console.log(account.state)
      return account.state; // If account exists, state will be non-empty
    } catch (error) {
      console.error(`Error checking account: ${error}`);
      return false;
    }
}

// connect to sqlite database and check if the tuple accountId and telegramId exist
async function checkAddressRegistered(accountId, telegramId){

    const sqlite3 = require('sqlite3').verbose();
    const db = new sqlite3.Database(DB_FILE);

    db.serialize(() => {
        db.get(`SELECT * FROM users WHERE wallet = '${accountId}' AND telegram_id = '${telegramId}'`, (err, row) => {
            if(err) console.error(err.message);
            if(row) {
                db.close();
                return true;
            } else {
                db.close();
                return false;
            }
        });
    });

}

async function registerAddress(accountId, telegramId) {
    const sqlite3 = require('sqlite3').verbose();
    const db = new sqlite3.Database(DB_FILE);

    return new Promise((resolve, reject) => {
        db.serialize(() => {
            db.run(`INSERT INTO users (wallet, telegram_id) VALUES ('${accountId}', '${telegramId}')`, function(err) {
                db.close();
                if (err) {
                    console.error(err.message);
                    resolve(false);
                } else {
                    resolve(true);
                }
            });
        });
    });
    
}


// BOT CODE
const bot = new Telegraf(TELEGRAM_BOT_TOKEN)
const logStreamBot = fs.createWriteStream(LOG_FILE_BOT, { flags: 'a' });


// create loggin middleware
bot.use(async (ctx, next) => {
  logStreamBot.write(`${new Date().toISOString()} -- Processing update ${ctx.update.update_id}\n`);
  await next() // runs next middleware
  // runs after next middleware finishes
  logStreamBot.write(`${new Date().toISOString()} -- Processing update ${ctx.update.update_id}\n`);
})

bot.start((ctx) => ctx.reply('Welcome to NEAR DCA Bot. \n Send `/help` to get started. \n Send `/about` for more information about the project')) 
bot.help((ctx) => ctx.reply('Help message'))
bot.command('about', (ctx) => ctx.reply('About message'))
bot.hears('hi', (ctx) => ctx.reply('Hey there'))

// parse message register <address>
bot.command('register', async (ctx) => {
  const address = ctx.message.text.split(' ')[1]
  if (!address) {
    ctx.reply('Please provide a valid address')
    logStreamBot.write(`${new Date().toISOString()} -- Error: Invalid address provided: ${address}\n`)
    return
  }
  ctx.reply(`Registering new address: ${address}. Please wait...`)
  logStreamBot.write(`${new Date().toISOString()} -- Registering new address: ${address}. current telegram id: ${ctx.from.id}\n`)

  // check if address is a valid near address
  let isValid = await isValidNearAddress(config, address)
  if (!isValid) {
    ctx.reply('Please provide a valid address')
    logStreamBot.write(`${new Date().toISOString()} -- Error: Invalid address provided: ${address}\n`)
    return
  }
  else {
    logStreamBot.write(`${new Date().toISOString()} -- Valid address: ${isValid}\n`)
  }

  // check if address is already registered
  const registered = await checkAddressRegistered(address, ctx.from.id)
  if (registered) {
    ctx.reply('Address already registered')
    logStreamBot.write(`${new Date().toISOString()} -- Error: Address already registered: ${address}\n`)
    return
  }

  // register address in database
  const register = await registerAddress(address, ctx.from.id)
  if (register) {
    ctx.reply('Address registered')
    logStreamBot.write(`${new Date().toISOString()} -- Address registered: ${address}\n`)
  } else {
    ctx.reply('Error registering address')
    logStreamBot.write(`${new Date().toISOString()} -- Error registering address: ${address}, current telegram id: ${ctx.from.id}\n`)
  }
})

bot.launch()

// Enable graceful stop
process.once('SIGINT', () => bot.stop('SIGINT'))
process.once('SIGTERM', () => bot.stop('SIGTERM'))
