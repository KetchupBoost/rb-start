pub mod get {
    use chrono::Local;
    use serde_json::json;

    use crate::{database::Database, request::Request};

    pub fn bank_statement(request: Request) -> (u16, String) {
        let mut status = 200;
        let mut body = json!({}).to_string();
        let mut db = Database::new().unwrap();

        let account_id: i32 = request.account_id.parse::<i32>().unwrap_or(0);
        
        let account_query = format!(
            r#"
                SELECT 
                accounts.limit_amount AS limit_amount,
                balances.amount AS balance
                FROM accounts
                JOIN balances ON balances.account_id = accounts.id
                WHERE accounts.id = ${}
            "#,
            {account_id}
        ); 
        
        if let Ok(account) = db.conn.query_one(account_query.as_str(), &[&account_id]) {

            let limite: i32 = account.get("limit_amount");
            let total: i32 = account.get("balance");
    
            let dez_transacoes_query = r#"
                SELECT 
                    amount,
                    transaction_type,
                    description,
                    TO_CHAR(date, 'YYYY-MM-DD HH:MI:SS.US') AS date
                FROM transactions
                WHERE account_id = $1
                ORDER BY date DESC
                LIMIT 10
            "#;
            let dez_transacoes = db.conn.query(dez_transacoes_query, &[&account_id]).unwrap();
    
            let dez_transacoes_json: Vec<_> = dez_transacoes.into_iter().map(|transaction| {
                let amount: i32 = transaction.get("amount");
                let description: &str = transaction.get("description");
                let transaction_type: &str = transaction.get("transaction_type");
                let transaction_date: &str = transaction.get("date");
                
                json!({
                    "valor": amount,
                    "descricao": description,
                    "tipo": transaction_type,
                    "realizada_em": transaction_date,
                })
            }).collect();
            
            body = json!({
                "saldo": json!({
                    "limite": limite,
                    "total": total,
                    "data_extrato": Local::now().to_string(),
                }),
                "ultimas_transacoes": dez_transacoes_json,
            }).to_string();
        } else {
            status = 404
        }

        db.conn.close().unwrap();

        (status, body)
    }
    
    pub fn not_found() -> (u16, String) {
        (404, json!({}).to_string())
    }
}

pub mod post {
    use serde_json::json;

    use crate::{database::Database, request::Request};

    fn reached_limit(balance: i32, limit_amount: i32, amount: i32) -> bool {
        if (balance - amount) > limit_amount {
            return false
        }

        return (balance - amount).abs() > limit_amount
    }

    pub fn transaction(request: Request) -> (u16, String) {
        let mut status = 200;
        let mut body = json!({}).to_string();

        let mut db = Database::new().unwrap();
        let mut db_transaction = db.conn.transaction().unwrap();

        let account_id: i32 = request.account_id.parse::<i32>().unwrap_or(0);
        
        let account_query = format!(
            r#"
                SELECT 
                accounts.limit_amount AS limit_amount,
                balances.amount AS balance
                FROM accounts
                JOIN balances ON balances.account_id = accounts.id
                WHERE accounts.id = ${}
            "#,
            account_id
        ); 

        if let Ok(account) = db_transaction.query_one(&account_query, &[&account_id]) {
            let amount = request.body.valor.parse::<i32>().unwrap();
            let type_tsc = request.body.tipo.as_str();
            let description = request.body.descricao.as_str();

            let limit_amount: i32 = account.get("limit_amount");
            let balance: i32 = account.get("balance: i32");

            if amount == 0 
                || !vec!["c", "d"].contains(&type_tsc) 
                || description.is_empty() 
                || description.len() > 10 
                ||  (type_tsc == "d" && reached_limit(balance, limit_amount, amount)) 
            {
                status = 422;
            } else {
                let insert_stmt = r#"
                    INSERT INTO transactions (account_id, amount, transaction_type, description)
                    VALUES ($1, $2, $3, $4)
                "#;

                let _ = db_transaction.execute(insert_stmt, &[&account_id, &amount, &type_tsc, &description]).unwrap();

                if type_tsc == "c" {
                    let update_stmt = r#"
                        UPDATE accounts 
                        SET balance = balance + $2
                        WHERE accounts.id = $1
                    "#;

                    let _ = db_transaction.execute(update_stmt, &[&account_id, &amount]).unwrap();
                } else {
                    let update_stmt = r#"
                        UPDATE accounts 
                        SET balance = balance - $2
                        WHERE accounts.id = $1
                    "#;

                    let _ = db_transaction.execute(update_stmt, &[&account_id, &amount]).unwrap();
                }

                let account = db_transaction.query_one(&account_query, &[&account_id]).unwrap();
                let limit_amount: i32 = account.get("limit_amount");
                let balance: i32 = account.get("balance");


                body = json!({
                    "limite": limit_amount,
                    "saldo": balance
                }).to_string();
            }
        } else {
            status = 404;   
        }

        db_transaction.commit().unwrap();

        (status, body)
    }
}