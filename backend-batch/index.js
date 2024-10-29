const nearAPI = require("near-api-js");
const { connect, keyStores } = nearAPI;
const path = require('path');
const fs = require('fs');
const { log } = require("console");
const { Telegraf } = require('telegraf')
const { message } = require('telegraf/filters')
const dotenv = require('dotenv');
const { env } = require("process");
const { isValidNearAddress, checkAddressRegistered, getRegisteredAddresses, deleteRegisteredAddress, registerAddress, registerConversion, getTelegramUsers, getConversions, getLatestConversion, getNearAccountBalance } = require('./utils');


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
    
        const responseView = await account.viewFunction({
            contractId: CONTRACT_ID,
            methodName: 'can_swap',
            args: {
            // Change method arguments go here
            },
        });
        logStream.write(`Read function result: ${JSON.stringify(responseView)}\n`);

        // if can't swap, exit
        if(responseView === false) {
            logStream.write(`Can't swap, exit\n`);
            return;
        }

        const response = await account.functionCall({
            contractId: CONTRACT_ID,
            methodName: 'swap',
            args: {
            // Change method arguments go here
            },
            gas: '300000000000000', // Adjust gas accordingly
            attachedDeposit: '1', // Optional: attach NEAR tokens if needed
        });

        // parse the response and for each receipts_outcome check if has outcome log
        const receipts_outcome = response.receipts_outcome
        for (const outcome of receipts_outcome) {
            if (outcome.outcome.logs.length > 0) {
                for (const log of outcome.outcome.logs) {
                    if (log.startsWith('<swapLog>')) {
                      let logVal = log.replace('<swapLog> ', '');
                      logStream.write(`Swap logs: ${logVal}\n`);
                      const json = JSON.parse(logVal);
                      logStream.write(`Swap logs: ${outcome.outcome.logs}\n`);
                      registerConversion(DB_FILE, outcome.id, json.user, json.source_amount, json.target_amount, json.source, json.target);

                      // check if a user subscribed to telegram notification for the user address
                      const registeredAddresses = await getTelegramUsers(DB_FILE, json.user);

                      logStream.write(`Registered addresses: ${JSON.stringify(registeredAddresses)}\n`);

                      // if registered, send telegram notification
                      if(registeredAddresses.length > 0) {
                        const bot = new Telegraf(TELEGRAM_BOT_TOKEN)
                        registeredAddresses.forEach(registeredAddress => {
                            logStream.write(`User ${registeredAddress.telegram_id} subscribed to telegram notification for ${json.user}... sending message\n`);
                            bot.telegram.sendMessage(registeredAddress.telegram_id, `🔄💸 Conversion Alert! 💸🔄\n\n👤 User: ${json.user}\n💰 ${json.source_amount} ${json.source} ➡️ ${json.target_amount} ${json.target}\n🚀`);
                        });
                      }

                    }
                }

            }
        }
        
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
bot.help((ctx) => ctx.reply(`
Available commands:

/register <address> - Register a new address to track
/unregister <address> - Unregister an address
/list - List all registered addresses
/swaps <address> - Get a list of swaps for an address
/last_swap <address> - Get the last swap for an address
/check <address> - Get the onchain status for an address
/about - About message
/help - Help message
`))
bot.command('about', (ctx) => ctx.reply('About message'))


// parse message register <address>
bot.command('register', async (ctx) => {
  const address = ctx.message.text.split(' ')[1]
  if (!address) {
    ctx.reply('Please provide a valid address. Send `/register <address>`')
    logStreamBot.write(`${new Date().toISOString()} -- Error: Invalid address provided: ${address}\n`)
    return
  }
  ctx.reply(`Registering new address: ${address}. Please wait...`)
  logStreamBot.write(`${new Date().toISOString()} -- Registering new address: ${address}. current telegram id: ${ctx.from.id}\n`)

  // check if address is a valid near address
  let isValid = await isValidNearAddress(config, address)
  console.log(isValid)
  if (!isValid) {
    ctx.reply('${address} is not a valid NEAR address. Please provide a valid address.')
    logStreamBot.write(`${new Date().toISOString()} -- Error: Invalid address provided: ${address}\n`)
    return
  }
  else {
    logStreamBot.write(`${new Date().toISOString()} -- Valid address: ${isValid}\n`)
  }

  // check if address is already registered
  let registered = await checkAddressRegistered(DB_FILE, address, ctx.from.id)
  if (registered) {
    ctx.reply('Address ${address} already registered')
    logStreamBot.write(`${new Date().toISOString()} -- Error: Address already registered: ${address}\n`)
    return
  }

  // register address in database
  let register = await registerAddress(DB_FILE, address, ctx.from.id)
  if (register) {
    ctx.reply('Address registered')
    logStreamBot.write(`${new Date().toISOString()} -- Address registered: ${address}\n`)
  } else {
    ctx.reply('Error registering address')
    logStreamBot.write(`${new Date().toISOString()} -- Error registering address: ${address}, current telegram id: ${ctx.from.id}\n`)
  }
})

bot.command('list', async (ctx) => {
  const addresses = await getRegisteredAddresses(DB_FILE, ctx.from.id)
  if (addresses.length === 0) {
    ctx.reply('No addresses registered')
    logStreamBot.write(`${new Date().toISOString()} -- Error: No addresses registered\n`)
    return
  }
  ctx.reply(`Registered addresses:\n${addresses.map(address => address.wallet).join('\n')}`)
  logStreamBot.write(`${new Date().toISOString()} -- Registered addresses: ${addresses.map(address => address.wallet).join(', ')}\n`)
})

bot.command('unregister', async (ctx) => {
  const address = ctx.message.text.split(' ')[1]
  if (!address) {
    ctx.reply('Please provide a valid address. Send `/delete <address>`')
    logStreamBot.write(`${new Date().toISOString()} -- Error: Invalid address provided: ${address}\n`)
    return
  }
  ctx.reply(`Deleting address: ${address}. Please wait...`)
  logStreamBot.write(`${new Date().toISOString()} -- Deleting address: ${address}\n`)

  // check if address is a valid near address
  let isValid = await isValidNearAddress(config, address)
  if (!isValid) {
    ctx.reply('${address} is not a valid NEAR address. Please provide a valid address')
    logStreamBot.write(`${new Date().toISOString()} -- Error: Invalid address provided: ${address}\n`)
    return
  }
  else {
    logStreamBot.write(`${new Date().toISOString()} -- Valid address: ${isValid}\n`)
  }

  // check if address is already registered
  let registered = await checkAddressRegistered(DB_FILE, address, ctx.from.id)
  if (!registered) {
    ctx.reply('Address ${address} not registered')
    logStreamBot.write(`${new Date().toISOString()} -- Error: Address not registered: ${address}\n`)
    return
  }

  // delete address from database
  let deleteAddress = await deleteRegisteredAddress(DB_FILE, address, ctx.from.id)
  if (deleteAddress) {
    ctx.reply('Address deleted')
    logStreamBot.write(`${new Date().toISOString()} -- Address deleted: ${address}\n`)
  } else {
    ctx.reply('Error deleting address')
    logStreamBot.write(`${new Date().toISOString()} -- Error deleting address: ${address}\n`)
  }
})

bot.command('swaps', async (ctx) => {
  const address = ctx.message.text.split(' ')[1]
  if (!address) {
    ctx.reply('Please provide a valid address. Send `/swap <address>`')
    logStreamBot.write(`${new Date().toISOString()} -- Error: Invalid address provided: ${address}\n`)
    return
  }
  ctx.reply(`Getting the list of swaps for address: ${address}. Please wait...`)
  logStreamBot.write(`${new Date().toISOString()} -- Swapping address: ${address}\n`)

  // check if address is a valid near address
  let isValid = await isValidNearAddress(config, address)
  if (!isValid) {
    ctx.reply('${address} is not a valid NEAR address. Please provide a valid address')
    logStreamBot.write(`${new Date().toISOString()} -- Error: Invalid address provided: ${address}\n`)
    return
  }
  else {
    logStreamBot.write(`${new Date().toISOString()} -- Valid address: ${isValid}\n`)
  }

  // check if address is already registered
  let registered = await checkAddressRegistered(DB_FILE, address, ctx.from.id)
  if (!registered) {
    ctx.reply('Address ${address} not registered')
    logStreamBot.write(`${new Date().toISOString()} -- Error: Address not registered: ${address}\n`)
    return
  }

  // get list of swaps
  let swaps = await getConversions(DB_FILE, address)
  if (swaps) {
    const formattedSwaps = swaps.map(swap => {
      return `User: ${swap.user}, Source: ${swap.source}, Source Amount: ${swap.source_amount}, Target: ${swap.target}, Target Amount: ${swap.target_amount}`;
    }).join('\n');
    ctx.reply(formattedSwaps);
    logStreamBot.write(`${new Date().toISOString()} -- Swaps: ${swaps}\n`)
  } else {
    ctx.reply('Error getting swaps')
    logStreamBot.write(`${new Date().toISOString()} -- Error getting swaps: ${address}\n`)
  }
})

bot.command('last_swap', async (ctx) => {
  const address = ctx.message.text.split(' ')[1]
  if (!address) {
    ctx.reply('Please provide a valid address. Send `/last_swaps <address>`')
    logStreamBot.write(`${new Date().toISOString()} -- Error: Invalid address provided: ${address}\n`)
    return
  }
  ctx.reply(`Getting the last swap for address: ${address}. Please wait...`)
  logStreamBot.write(`${new Date().toISOString()} -- Last swaps address: ${address}\n`)

  // check if address is a valid near address
  let isValid = await isValidNearAddress(config, address)
  if (!isValid) {
    ctx.reply('${address} is not a valid NEAR address. Please provide a valid address')
    logStreamBot.write(`${new Date().toISOString()} -- Error: Invalid address provided: ${address}\n`)
    return
  }
  else {
    logStreamBot.write(`${new Date().toISOString()} -- Valid address: ${isValid}\n`)
  }

  // check if address is already registered
  let registered = await checkAddressRegistered(DB_FILE, address, ctx.from.id)
  if (!registered) {
    ctx.reply('Address ${address} not registered')
    logStreamBot.write(`${new Date().toISOString()} -- Error: Address not registered: ${address}\n`)
    return
  }

  // get last swap
  let lastSwap = await getLatestConversion(DB_FILE, address)
  if (lastSwap) {
    ctx.reply(`Last Swap Details:\nAccount ID: ${lastSwap.account_id}\nSource Amount: ${lastSwap.amount_source}\nDestination Amount: ${lastSwap.amount_dest}\nSource Token: ${lastSwap.token_source}\nDestination Token: ${lastSwap.token_dest}\nTransaction ID: ${lastSwap.transaction_id}\nDate: ${lastSwap.date}`)
    logStreamBot.write(`${new Date().toISOString()} -- Last swap: ${lastSwap}\n`)
  } else {
    ctx.reply('Error getting last swap')
    logStreamBot.write(`${new Date().toISOString()} -- Error getting last swap: ${address}\n`)
  }
})

bot.command('check', async (ctx) => {
  const address = ctx.message.text.split(' ')[1]
  if (!address) {
    ctx.reply('Please provide a valid address. Send `/check <address>`')
    logStreamBot.write(`${new Date().toISOString()} -- Error: Invalid address provided: ${address}\n`)
    return
  }
  ctx.reply(`Checking address: ${address}. Please wait...`)
  logStreamBot.write(`${new Date().toISOString()} -- Checking address: ${address}\n`)

  // check if address is a valid near address
  let isValid = await isValidNearAddress(config, address)
  if (!isValid) {
    ctx.reply('${address} is not a valid NEAR address. Please provide a valid address')
    logStreamBot.write(`${new Date().toISOString()} -- Error: Invalid address provided: ${address}\n`)
    return
  }
  else {
    logStreamBot.write(`${new Date().toISOString()} -- Valid address: ${isValid}\n`)
  }

  let status = await getNearAccountBalance(config, CONTRACT_ID, address)
  if (status) {
    ctx.reply(`Account status:\nAmount per swap: ${status.amount_per_swap}\nSwap interval: ${status.swap_interval}\nLast swap timestamp: ${status.last_swap_timestamp}\nTarget amount: ${status.total_swapped}\nPaused: ${status.pause}`)
    logStreamBot.write(`${new Date().toISOString()} -- Status: ${status}\n`)
  } else {
    ctx.reply('Error getting status')
    logStreamBot.write(`${new Date().toISOString()} -- Error getting status: ${address}\n`)
  }
})

bot.launch()

// Enable graceful stop
process.once('SIGINT', () => bot.stop('SIGINT'))
process.once('SIGTERM', () => bot.stop('SIGTERM'))
