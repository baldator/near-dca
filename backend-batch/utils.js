const nearAPI = require("near-api-js");
const { connect } = nearAPI;

async function isValidNearAddress(nearConfig, accountId) {
  try {
    const near = await connect(nearConfig);
    const account = await near.account(accountId);
    const state = await account.state();
    return Object.keys(state).length > 0;
  } catch (error) {
    console.error(`Error checking account: ${error}`);
    return false;
  }
}

async function getNearAccountBalance(nearConfig, contractId, accountId) {
  try {
    const near = await connect(nearConfig);
    const account = await near.account(accountId);
    const responseView = await account.viewFunction({
        contractId: contractId,
        methodName: 'get_user',
        args: {
            user: accountId
        },
    });

    console.log(responseView);
    return responseView;
  } catch (error) {
    console.error(`Error getting account balance: ${error}`);
    return false;
  }
}

// connect to sqlite database and check if the tuple accountId and telegramId exist
async function checkAddressRegistered(dbFile, accountId, telegramId){

    const sqlite3 = require('sqlite3').verbose();
    const db = new sqlite3.Database(dbFile);

    return new Promise((resolve, reject) => {
        db.serialize(() => {
            db.get(`SELECT * FROM users WHERE wallet = '${accountId}' AND telegram_id = '${telegramId}'`, (err, row) => {
                if(err) {
                    console.error(err.message);
                    reject(false);
                } else {
                    db.close();
                    resolve(row !== undefined);
                }
            });
        });
    });

}

// connect to sqlite database and get all registered addresses for current telegram user
async function getRegisteredAddresses(dbFile, telegramId) {
    const sqlite3 = require('sqlite3').verbose();
    const db = new sqlite3.Database(dbFile);

    return new Promise((resolve, reject) => {
        db.serialize(() => {
            db.all(`SELECT * FROM users WHERE telegram_id = '${telegramId}'`, (err, rows) => {
                if(err) {
                    console.error(err.message);
                    reject(false);
                } else {
                    db.close();
                    resolve(rows);
                }
            });
        });
    });
}

// connect to sqlite database and get all telegram users for current near account
async function getTelegramUsers(dbFile, accountId) {
    const sqlite3 = require('sqlite3').verbose();
    const db = new sqlite3.Database(dbFile);

    return new Promise((resolve, reject) => {
        db.serialize(() => {
            db.all(`SELECT * FROM users WHERE wallet = '${accountId}'`, (err, rows) => {
                if(err) {
                    console.error(err.message);
                    reject(false);
                } else {
                    db.close();
                    resolve(rows);
                }
            });
        });
    });
}

// connect to sqlite database and delete tuple accountId and telegramId
async function deleteRegisteredAddress(dbFile, accountId, telegramId) {
    const sqlite3 = require('sqlite3').verbose();
    const db = new sqlite3.Database(dbFile);

    return new Promise((resolve, reject) => {
        db.serialize(() => {
            db.run(`DELETE FROM users WHERE wallet = '${accountId}' AND telegram_id = '${telegramId}'`, function(err) {
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

async function registerAddress(dbFile, accountId, telegramId) {
    const sqlite3 = require('sqlite3').verbose();
    const db = new sqlite3.Database(dbFile);

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

async function registerConversion(dbFile, transaction_id, account_id, amount_source, amount_dest, token_source, token_dest) {
    const sqlite3 = require('sqlite3').verbose();
    const db = new sqlite3.Database(dbFile);

    return new Promise((resolve, reject) => {
        db.serialize(() => {
            db.run(`INSERT INTO conversion (date, transaction_id, account_id, amount_source, amount_dest, token_source, token_dest) VALUES (datetime('now'), '${transaction_id}', '${account_id}', '${amount_source}', '${amount_dest}', '${token_source}', '${token_dest}')`, function(err) {
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

async function getConversions(dbFile, accountId) {
    const sqlite3 = require('sqlite3').verbose();
    const db = new sqlite3.Database(dbFile);

    return new Promise((resolve, reject) => {
        db.serialize(() => {
            db.all(`SELECT * FROM conversion WHERE account_id = '${accountId}' ORDER BY date DESC LIMIT 10;`, (err, rows) => {
                if(err) {
                    console.error(err.message);
                    reject(false);
                } else {
                    db.close();
                    resolve(rows);
                }
            });
        });
    });
}

async function getLatestConversion(dbFile, accountId) {
    const sqlite3 = require('sqlite3').verbose();
    const db = new sqlite3.Database(dbFile);

    return new Promise((resolve, reject) => {
        db.serialize(() => {
            db.all(`SELECT * FROM conversion WHERE account_id = '${accountId}' ORDER BY date DESC LIMIT 1`, (err, rows) => {
                if(err) {
                    console.error(err.message);
                    reject(false);
                } else {
                    db.close();
                    resolve(rows);
                }
            });
        });
    });
}



module.exports = {
    isValidNearAddress,
    checkAddressRegistered,
    getRegisteredAddresses,
    deleteRegisteredAddress,
    registerAddress,
    registerConversion,
    getTelegramUsers,
    getConversions,
    getLatestConversion,
    getNearAccountBalance
}