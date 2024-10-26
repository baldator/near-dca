const nearAPI = require("near-api-js");
const { connect, keyStores } = nearAPI;
const path = require('path');
const fs = require('fs');


const NEAR_NET = 'testnet';
const CONTRACT_ID = 'test.dca-near.testnet';
const ACCOUNT_ID = 'dca-near.testnet';
const LOG_FILE = `./logs/dca-batch-${new Date().toISOString().split('T')[0]}.log`;

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

const logStream = fs.createWriteStream(LOG_FILE, { flags: 'a' });
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

main().catch((error) => {
     logStream.write(`Error calling main function: ${error}\n`);
     console.error(error);
});