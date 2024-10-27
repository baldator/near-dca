// create a squite database to store the conversion logs
const sqlite3 = require('sqlite3').verbose();
const db = new sqlite3.Database('./data/dca-batch.db');

db.serialize(() => {
    db.run('CREATE TABLE IF NOT EXISTS conversion (id INTEGER PRIMARY KEY AUTOINCREMENT, date TIMESTAMP, transaction_id varchar(255), account_id varchar(255), amount_per_swap varchar(255), amount_source real, amount_dest real, token_source varchar(255), token_dest varchar(255));');
    db.run('CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY AUTOINCREMENT, wallet varchar(255), telegram_id varchar(255), UNIQUE(wallet, telegram_id) ON CONFLICT IGNORE);');
})