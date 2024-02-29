use std::{collections::HashMap, io::{BufRead, BufReader, Read, Write}, net::TcpListener};
use chrono::Local;
use postgres::{Client, NoTls};
use serde_json::{json, Value};
use regex::Regex;


fn main() {
    let listener = TcpListener::bind("0.0.0.0:3000").unwrap();
    println!("Listening on the port 3000");

    for client in listener.incoming() {
        println!("Client connected!");
        println!("=====================");
        // * Reading the Request
        let mut client = client.unwrap();
        let mut reader = BufReader::new(&client);
        let mut headline = String::new();
        let mut params: HashMap<&str, String> = HashMap::new();

        let _ = reader.read_line(&mut headline);
        
        let headline_pattern = Regex::new(r"^(GET|POST)\s\/clientes\/(\d+)\/(.*?)\sHTTP.*?").unwrap();
        let captures = headline_pattern.captures(&headline).unwrap();
        let verb = captures.get(1).unwrap().as_str();
        let account_id = captures.get(2).unwrap().as_str();
        let suffix = captures.get(3).unwrap().as_str();

        let request_constraint = format!("{verb} /clientes/:id/{suffix}");
        
        params.insert("id", account_id.to_string());

        let mut content_length: u64 = 0;
        for line in reader.by_ref().lines() {
            let line = line.unwrap();

            if line.contains("Content-Length") {
                let splited_line: Vec<_> = line.split(":").collect();
                content_length = splited_line[1].trim().parse::<u64>().unwrap();
            }
            
            if line.is_empty() {
                break;
            }
            continue;
        }


        // * Reading Body
        if content_length > 0 {
            let mut body = String::new();
            let _ = reader
                .take(content_length)
                .read_to_string(&mut body);
    
            let parsed: Value = serde_json::from_str(&body).unwrap();
            let valor = parsed["valor"].to_string();
            let tipo = parsed["tipo"].as_str().unwrap().to_string();
            let descricao = parsed["descricao"].as_str().unwrap().to_string();

            params.insert("valor", valor);
            params.insert("tipo", tipo);
            params.insert("descricao", descricao);
        }

        println!("Params: {params:?}");

        // * Creating a response
        let line_break = "\r\n";
        let mut response_body = json!({"message": "Empty Body"});
        let mut status = "200 OK";
        
        // let dez_transacoes = vec![
        //     json!({"valor": 200, "tipo": "d", "realizada_em": "2020-01-01", "descricao": "Qualquer Descricao"}),
        //     json!({"valor": 100, "tipo": "c", "realizada_em": "2020-01-01", "descricao": "Qualquer Descricao"})
        // ];

        match request_constraint.as_str() {
            "GET /clientes/:id/extrato" => {
                let account_id = params["id"].parse::<i32>().unwrap();
                
                let mut db = Client::connect("host=localhost user=postgres password=postgres dbname=postgres",NoTls).unwrap();

                let account_query = r#"
                    SELECT 
                        accounts.limit_amount AS limit_amount,
                        balances.amount AS balance
                    FROM accounts
                    JOIN balances ON balances.account_id = accounts.id
                    WHERE accounts.id = $1
                "#;

                let account = db.query_one(account_query, &[&account_id]).unwrap();
                let limite: i32 = account.get("limit_amount");
                let total: i32 = account.get("balance");

                let dez_transacoes_query = r#"
                    SELECT 
                        amount,
                        transaction_type,
                        description,
                        TO_CHAR(date, 'YYYY-MM-DD') AS date
                    FROM transactions
                    WHERE account_id = $1
                    ORDER BY date DESC
                    LIMIT 10
                "#;
                let dez_transacoes = db.query(dez_transacoes_query, &[&account_id]).unwrap();

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
                
                response_body = json!({
                    "saldo": json!({
                        "limite": limite,
                        "total": total,
                        "data_extrato": Local::now().to_string(),
                    }),
                    "ultimas_transacoes": dez_transacoes_json,
                });
            },
            "POST /clientes/:id/transacoes" => {
                let account_id = params["id"].parse::<i32>().unwrap();
                let amount = params["valor"].parse::<i32>().unwrap();
                let transaction_type = params["tipo"].as_str();
                let description = params["descricao"].as_str();
                
                let mut db = Client::connect("host=localhost user=postgres password=postgres dbname=postgres",NoTls).unwrap();

                let insert_query = r#"
                    INSERT INTO transactions (account_id, amount, transaction_type, description)
                        VALUES ($1, $2, $3, $4)
                "#;
                
                let _ = db.execute(insert_query, &[&account_id, &amount, &transaction_type, &description]);
                
                match transaction_type {
                    "c" => {
                        let insert_stmt = r#"
                            UPDATE balances
                            SET amount = amount + $2
                            WHERE account_id = $1
                        "#;
                        
                        let _ = db.execute(insert_stmt, &[&account_id, &amount]).unwrap();
                    },
                    "d" => {
                        let insert_stmt = r#"
                            UPDATE balances
                            SET amount = amount - $2
                            WHERE account_id = $1
                        "#;
                        
                        let _ = db.execute(insert_stmt, &[&account_id, &amount]).unwrap();
                    }
                    _ => panic!("Transaction Type is Incorrect")
                };

                let account_query = r#"
                    SELECT 
                        accounts.limit_amount AS limit_amount,
                        balances.amount AS balance
                    FROM accounts
                    JOIN balances ON balances.account_id = accounts.id
                    WHERE accounts.id = $1
                "#;

                let account = db.query_one(account_query, &[&account_id]).unwrap();
                let limite: i32 = account.get("limit_amount");
                let total: i32 = account.get("balance");

                response_body = json!({
                    "limite": limite,
                    "saldo": total,
                });
            },
            _ => {
                status = "404 FAIL";
            }
        }

        let response = format!("HTTP/1.1 {status}{line_break}Content-type: application/json{line_break}{line_break}{response_body}");

        client.write_all(response.as_bytes()).unwrap();
    }
}
