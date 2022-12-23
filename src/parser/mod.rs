use std::collections::HashMap;
use std::mem::swap;

use crate::lexer::Lexer;
use crate::parser::expression::{
    ArrayLiteral, BooleanLiteral, CallExpression, Expression, FunctionLiteral, HashLiteral,
    Identifier, IfExpression, InfixExpression, IntegerLiteral, PrefixExpression, StringLiteral,
};
use crate::parser::program::Program;
use crate::parser::statement::{
    BlockStatement, ExpressionStatement, LetStatement, ReturnStatement, Statement,
};
use crate::token::Token;

pub mod expression;
pub mod node;
pub mod program;
pub mod statement;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
enum PrecedenceType {
    LOWEST,
    EQUALS,
    LESSGREATER,
    SUM,
    PRODUCT,
    PREFIX,
    CALL,
}

pub struct Parser {
    lexer: Lexer,
    errors: Vec<String>,
    cur_token: Token,
    peek_token: Token,
}

impl Parser {
    pub fn new(mut lexer: Lexer) -> Self {
        let cur_token = lexer.next_token();
        let peek_token = lexer.next_token();
        Parser {
            lexer,
            errors: Vec::new(),
            cur_token,
            peek_token,
        }
    }

    fn next_token(&mut self) {
        swap(&mut self.cur_token, &mut self.peek_token);
        self.peek_token = self.lexer.next_token();
    }

    pub fn parse_program(&mut self) -> Program {
        let mut program = Program {
            statements: Vec::new(),
        };

        while !matches!(self.cur_token, Token::EOF) {
            if let Some(statement) = self.parse_statement() {
                program.statements.push(statement);
            }
            self.next_token();
        }

        program
    }

    fn parse_statement(&mut self) -> Option<Statement> {
        match &self.cur_token {
            Token::LET => self.parse_let_statement(),
            Token::RETURN => self.parse_return_statement(),
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_let_statement(&mut self) -> Option<Statement> {
        let token = self.cur_token.clone();

        if matches!(self.peek_token, Token::IDENT(_)) {
            self.next_token()
        } else {
            self.errors.push(format!(
                "expected next token to be Token::IDENT, got {} instead",
                self.peek_token
            ));
            return None;
        }

        let name = Identifier {
            token: self.cur_token.clone(),
        };

        if matches!(self.peek_token, Token::ASSIGN) {
            self.next_token()
        } else {
            self.errors.push(format!(
                "expected next token to be Token::ASSIGN, got {} instead",
                self.peek_token
            ));
            return None;
        }

        self.next_token();
        let value = self.parse_expression(PrecedenceType::LOWEST);

        if matches!(self.peek_token, Token::SEMICOLON) {
            self.next_token();
        }

        Some(Statement::LetStatement(LetStatement {
            token,
            identifier: name,
            value,
        }))
    }

    fn parse_return_statement(&mut self) -> Option<Statement> {
        let token = self.cur_token.clone();

        self.next_token();
        let value = self.parse_expression(PrecedenceType::LOWEST);
        if matches!(self.peek_token, Token::SEMICOLON) {
            self.next_token();
        }

        Some(Statement::ReturnStatement(ReturnStatement { token, value }))
    }

    fn parse_expression_statement(&mut self) -> Option<Statement> {
        let token = self.cur_token.clone();
        let expression = self.parse_expression(PrecedenceType::LOWEST);

        let exp_stmt = ExpressionStatement { token, expression };

        if matches!(self.peek_token, Token::SEMICOLON) {
            self.next_token();
        }

        Some(Statement::ExpressionStatement(exp_stmt))
    }

    fn parse_block_statement(&mut self) -> BlockStatement {
        let token = self.cur_token.clone();
        let mut statements = vec![];

        self.next_token();
        while !matches!(self.cur_token, Token::RBRACE | Token::EOF) {
            let stmt = self.parse_statement();
            if let Some(s) = stmt {
                statements.push(s)
            }
            self.next_token();
        }

        BlockStatement { token, statements }
    }

    fn parse_expression(&mut self, precedence: PrecedenceType) -> Option<Expression> {
        let mut left = match &self.cur_token {
            Token::IDENT(_) => self.parse_identifier(),
            Token::INT(int_string) => self.parse_integer_literal(int_string.to_string()),
            Token::TRUE => self.parse_bool_literal(true),
            Token::FALSE => self.parse_bool_literal(false),
            Token::LPAREN => self.parse_grouped_expression(),
            Token::IF => self.parse_if_expression(),
            Token::FUNCTION => self.parse_fn_expression(),
            Token::LBRACKET => self.parse_array_literal(),
            Token::LBRACE => self.parse_hash_literal(),
            Token::BANG => self.parse_prefix_expression(),
            Token::MINUS => self.parse_prefix_expression(),
            Token::STRING(s) => self.parse_string_literal(),
            _ => {
                self.errors.push(format!(
                    "no prefix parse function for {:?} found",
                    self.cur_token
                ));
                None
            }
        }?;

        while !matches!(self.peek_token, Token::SEMICOLON)
            && precedence < Self::get_precedence(&self.peek_token)
        {
            if matches!(
                self.peek_token,
                Token::PLUS
                    | Token::MINUS
                    | Token::SLASH
                    | Token::ASTERISK
                    | Token::EQ
                    | Token::NEQ
                    | Token::LT
                    | Token::GT
            ) {
                self.next_token();
                left = self.parse_infix_expression(left)?;
            } else if matches!(self.peek_token, Token::LPAREN) {
                self.next_token();
                left = self.parse_call_expression(left)?;
            } else {
                break;
            }
        }

        Some(left)
    }

    fn parse_identifier(&mut self) -> Option<Expression> {
        Some(Expression::Identifier(Identifier {
            token: self.cur_token.clone(),
        }))
    }

    fn parse_integer_literal(&mut self, int_string: String) -> Option<Expression> {
        let token = self.cur_token.clone();
        let value_result = int_string.parse::<i64>();
        match value_result {
            Ok(value) => Some(Expression::IntegerLiteral(IntegerLiteral { token, value })),
            Err(_) => {
                self.errors
                    .push(format!("count not parse {:?} as integer", token));
                None
            }
        }
    }

    fn parse_bool_literal(&mut self, value: bool) -> Option<Expression> {
        Some(Expression::BooleanLiteral(BooleanLiteral {
            token: self.cur_token.clone(),
            value,
        }))
    }

    fn parse_string_literal(&mut self) -> Option<Expression> {
        Some(Expression::StringLiteral(StringLiteral {
            token: self.cur_token.clone(),
            value: self.cur_token.literal().to_string(),
        }))
    }

    fn parse_grouped_expression(&mut self) -> Option<Expression> {
        self.next_token();

        let exp = self.parse_expression(PrecedenceType::LOWEST);
        if matches!(self.peek_token, Token::RPAREN) {
            self.next_token();
            exp
        } else {
            None
        }
    }

    fn parse_if_expression(&mut self) -> Option<Expression> {
        let token = self.cur_token.clone();

        if matches!(self.peek_token, Token::LPAREN) {
            self.next_token();
        } else {
            return None;
        }

        self.next_token();
        let condition = self.parse_expression(PrecedenceType::LOWEST)?;

        if matches!(self.peek_token, Token::RPAREN) {
            self.next_token();
        } else {
            return None;
        }

        if matches!(self.peek_token, Token::LBRACE) {
            self.next_token();
        } else {
            return None;
        }

        let consequence = self.parse_block_statement();

        let mut if_exp = IfExpression {
            token,
            condition: Box::new(condition),
            consequence,
            alternative: None,
        };

        if matches!(self.peek_token, Token::ELSE) {
            self.next_token();

            if matches!(self.peek_token, Token::LBRACE) {
                self.next_token();
            } else {
                return None;
            }

            if_exp.alternative = Some(self.parse_block_statement());
        }

        Some(Expression::IfExpression(if_exp))
    }

    fn parse_fn_expression(&mut self) -> Option<Expression> {
        let token = self.cur_token.clone();

        if matches!(self.peek_token, Token::LPAREN) {
            self.next_token();
        } else {
            return None;
        }

        let parameters = self.parse_fn_parameters()?;

        if matches!(self.peek_token, Token::LBRACE) {
            self.next_token();
        } else {
            return None;
        }

        let body = self.parse_block_statement();

        Some(Expression::FunctionLiteral(FunctionLiteral {
            token,
            parameters,
            body,
        }))
    }

    fn parse_fn_parameters(&mut self) -> Option<Vec<Identifier>> {
        let mut vec = vec![];

        if matches!(self.peek_token, Token::RPAREN) {
            self.next_token();
            return Some(vec);
        }

        self.next_token();

        vec.push(Identifier {
            token: self.cur_token.clone(),
        });

        while matches!(self.peek_token, Token::COMMA) {
            self.next_token();
            self.next_token();

            vec.push(Identifier {
                token: self.cur_token.clone(),
            });
        }

        if matches!(self.peek_token, Token::RPAREN) {
            self.next_token();
        } else {
            return None;
        }

        Some(vec)
    }

    fn parse_array_literal(&mut self) -> Option<Expression> {
        let token = self.cur_token.clone();
        let elements = self.parse_expression_list(Token::RBRACKET)?;
        Some(Expression::ArrayLiteral(ArrayLiteral { token, elements }))
    }

    fn parse_hash_literal(&mut self) -> Option<Expression> {
        let token = self.cur_token.clone();
        let mut paris = HashMap::new();
        while !matches!(self.peek_token, Token::RBRACE) {
            self.next_token();
            let key = self.parse_expression(PrecedenceType::LOWEST)?;

            if matches!(self.peek_token, Token::COLON) {
                self.next_token();
            } else {
                return None;
            }

            self.next_token();
            let value = self.parse_expression(PrecedenceType::LOWEST)?;
            paris.insert(key, value);

            if !matches!(self.peek_token, Token::RBRACE) {
                if matches!(self.peek_token, Token::COMMA) {
                    self.next_token();
                } else {
                    return None;
                }
            }
        }

        if matches!(self.peek_token, Token::RBRACE) {
            self.next_token();
        } else {
            return None;
        }

        Some(Expression::HashLiteral(HashLiteral { token, paris }))
    }

    fn parse_expression_list(&mut self, token: Token) -> Option<Vec<Expression>> {
        let mut vec = vec![];

        if self.peek_token == token {
            self.next_token();
            return Some(vec);
        }

        self.next_token();
        vec.push(self.parse_expression(PrecedenceType::LOWEST)?);

        while matches!(self.peek_token, Token::COMMA) {
            self.next_token();
            self.next_token();
            vec.push(self.parse_expression(PrecedenceType::LOWEST)?);
        }

        if self.peek_token == token {
            self.next_token();
        } else {
            return None;
        }

        Some(vec)
    }

    fn parse_call_expression(&mut self, function: Expression) -> Option<Expression> {
        let token = self.cur_token.clone();
        let args = self.parse_expression_list(Token::RPAREN)?;
        Some(Expression::CallExpression(CallExpression {
            token,
            function: Box::new(function),
            args,
        }))
    }

    fn parse_prefix_expression(&mut self) -> Option<Expression> {
        let token = self.cur_token.clone();
        self.next_token();
        let right = self.parse_expression(PrecedenceType::PREFIX)?;
        Some(Expression::PrefixExpression(PrefixExpression {
            token,
            right: Box::new(right),
        }))
    }

    fn parse_infix_expression(&mut self, left: Expression) -> Option<Expression> {
        let token = self.cur_token.clone();
        let precedence = Self::get_precedence(&self.cur_token);

        self.next_token();
        let right = self.parse_expression(precedence)?;
        Some(Expression::InfixExpression(InfixExpression {
            token,
            left: Box::new(left),
            right: Box::new(right),
        }))
    }

    fn get_precedence(token: &Token) -> PrecedenceType {
        match token {
            Token::EQ | Token::NEQ => PrecedenceType::EQUALS,
            Token::LT | Token::GT => PrecedenceType::LESSGREATER,
            Token::PLUS | Token::MINUS => PrecedenceType::SUM,
            Token::SLASH | Token::ASTERISK => PrecedenceType::PRODUCT,
            Token::LPAREN => PrecedenceType::CALL,
            _ => PrecedenceType::LOWEST,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::Lexer;
    use crate::parser::expression::{CallExpression, Expression, FunctionLiteral};
    use crate::parser::statement::{ExpressionStatement, Statement};
    use crate::parser::Parser;
    use std::collections::HashMap;

    #[test]
    fn test_parser() {
        let input = "
let x = 5;
let y = 10;
return 5;
";
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);

        let program = parser.parse_program();
        if parser.errors.len() > 0 {
            println!(
                "parser.errors.len() = {:?}, {:?}",
                parser.errors.len(),
                parser.errors
            );
            assert_eq!(parser.errors.len(), 0);
            return;
        }

        if let Statement::LetStatement(s) = &program.statements[0] {
            assert_eq!(s.token.to_string(), "let");
            assert_eq!(s.identifier.token.to_string(), "x");
        } else {
            assert!(false);
        }

        if let Statement::LetStatement(s) = &program.statements[1] {
            assert_eq!(s.token.to_string(), "let");
            assert_eq!(s.identifier.token.to_string(), "y");
        } else {
            assert!(false);
        }

        if let Statement::ReturnStatement(s) = &program.statements[2] {
            assert_eq!(s.token.to_string(), "return");
        } else {
            assert!(false);
        }
    }

    #[test]
    fn test_identifier_expression() {
        let input = "foobar;";

        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program();

        if parser.errors.len() > 0 {
            println!("error: {:?}", parser.errors);
            return;
        }

        for s in &program.statements {
            println!("{}", s);
        }

        assert_eq!(program.statements.len(), 1);

        let exp_stmt = unwrap_to_expression_statement(&program.statements[0]).unwrap();
        assert_identifier(exp_stmt.expression.as_ref().unwrap(), "foobar");
    }

    #[test]
    fn test_integer_literal_expression() {
        let input = "5;";

        let mut parser = Parser::new(Lexer::new(input));
        let program = parser.parse_program();
        if parser.errors.len() > 0 {
            println!("{:?}", parser.errors);
            return;
        }

        for s in &program.statements {
            println!("{}", s);
        }

        assert_eq!(program.statements.len(), 1);

        let exp_stmt = unwrap_to_expression_statement(&program.statements[0]).unwrap();
        assert_integer_literal(exp_stmt.expression.as_ref().unwrap(), 5);
    }

    #[test]
    fn test_prefix_expression() {
        let prefix_tests = vec![
            ("!5", "!", Literal::NumberLiteral(5)),
            ("-15", "-", Literal::NumberLiteral(15)),
            ("!true", "!", Literal::BoolLiteral(true)),
            ("!false", "!", Literal::BoolLiteral(false)),
        ];

        for test in prefix_tests {
            let mut parser = Parser::new(Lexer::new(test.0));
            let program = parser.parse_program();
            if parser.errors.len() > 0 {
                println!("{:?}", parser.errors);
                assert!(false);
                continue;
            }

            for s in &program.statements {
                println!("{}", s);
            }
            assert_eq!(program.statements.len(), 1);
            let exp_stmt = unwrap_to_expression_statement(&program.statements[0]).unwrap();
            assert_prefix_expression(exp_stmt.expression.as_ref().unwrap(), test.1, test.2);
        }
    }

    #[test]
    fn test_infix_expression() {
        let infix_tests = vec![
            (
                "5 + 5;",
                Literal::NumberLiteral(5),
                "+",
                Literal::NumberLiteral(5),
            ),
            (
                "5 - 5;",
                Literal::NumberLiteral(5),
                "-",
                Literal::NumberLiteral(5),
            ),
            (
                "5 * 5;",
                Literal::NumberLiteral(5),
                "*",
                Literal::NumberLiteral(5),
            ),
            (
                "5 / 5;",
                Literal::NumberLiteral(5),
                "/",
                Literal::NumberLiteral(5),
            ),
            (
                "5 > 5;",
                Literal::NumberLiteral(5),
                ">",
                Literal::NumberLiteral(5),
            ),
            (
                "5 < 5;",
                Literal::NumberLiteral(5),
                "<",
                Literal::NumberLiteral(5),
            ),
            (
                "5 == 5;",
                Literal::NumberLiteral(5),
                "==",
                Literal::NumberLiteral(5),
            ),
            (
                "5 != 5;",
                Literal::NumberLiteral(5),
                "!=",
                Literal::NumberLiteral(5),
            ),
            (
                "true == true",
                Literal::BoolLiteral(true),
                "==",
                Literal::BoolLiteral(true),
            ),
            (
                "true != false",
                Literal::BoolLiteral(true),
                "!=",
                Literal::BoolLiteral(false),
            ),
            (
                "false == false",
                Literal::BoolLiteral(false),
                "==",
                Literal::BoolLiteral(false),
            ),
        ];

        for test in infix_tests {
            let mut parser = Parser::new(Lexer::new(test.0));
            let program = parser.parse_program();
            if parser.errors.len() > 0 {
                println!("{:?}", parser.errors);
                assert!(false);
                continue;
            }

            for s in &program.statements {
                println!("{}", s);
            }
            assert_eq!(program.statements.len(), 1);

            let exp_stmt = unwrap_to_expression_statement(&program.statements[0]).unwrap();
            assert_infix_expression(
                exp_stmt.expression.as_ref().unwrap(),
                test.1,
                test.2,
                test.3,
            );
        }
    }

    #[test]
    fn test_operator_precedence_parsing() {
        let test_vec = vec![
            ("-a * b", "((-a) * b)"),
            ("!-a", "(!(-a))"),
            ("a + b + c", "((a + b) + c)"),
            ("a + b - c", "((a + b) - c)"),
            ("a * b * c", "((a * b) * c)"),
            ("a * b / c", "((a * b) / c)"),
            ("a + b / c", "(a + (b / c))"),
            ("a + b * c + d / e - f", "(((a + (b * c)) + (d / e)) - f)"),
            ("3 + 4; -5 * 5", "(3 + 4)((-5) * 5)"),
            ("5 > 4 == 3 < 4", "((5 > 4) == (3 < 4))"),
            ("5 < 4 != 3 > 4", "((5 < 4) != (3 > 4))"),
            (
                "3 + 4 * 5 == 3 * 1 + 4 * 5",
                "((3 + (4 * 5)) == ((3 * 1) + (4 * 5)))",
            ),
            ("true", "true"),
            ("false", "false"),
            ("3 > 5 == false", "((3 > 5) == false)"),
            ("3 < 5 == true", "((3 < 5) == true)"),
            ("1 + (2 + 3) + 4", "((1 + (2 + 3)) + 4)"),
            ("(5 + 5) * 2", "((5 + 5) * 2)"),
            ("2 / (5 + 5)", "(2 / (5 + 5))"),
            ("-(5 + 5)", "(-(5 + 5))"),
            ("!(true == true)", "(!(true == true))"),
            ("a + add(b * c) + d", "((a + add((b * c))) + d)"),
            (
                "add(a, b, 1, 2 * 3, 4 + 5, add(6, 7 * 8))",
                "add(a, b, 1, (2 * 3), (4 + 5), add(6, (7 * 8)))",
            ),
            (
                "add(a + b + c * d / f + g)",
                "add((((a + b) + ((c * d) / f)) + g))",
            ),
        ];

        for test in test_vec {
            let mut parser = Parser::new(Lexer::new(test.0));
            let program = parser.parse_program();
            if parser.errors.len() > 0 {
                println!("{:?}", parser.errors);
                assert!(false);
                continue;
            }

            println!("{}", program);
            assert_eq!(program.to_string(), test.1);
        }
    }

    #[test]
    fn test_if_expression() {
        let input = "if (x < y) { x }";

        let mut parser = Parser::new(Lexer::new(input));
        let program = parser.parse_program();
        if parser.errors.len() > 0 {
            println!("{:?}", parser.errors);
            assert!(false);
            return;
        }

        println!("{}", program);
        assert_eq!(program.statements.len(), 1);

        if let Statement::ExpressionStatement(exp) = &program.statements[0] {
            if let Expression::IfExpression(if_exp) = exp.expression.as_ref().unwrap() {
                assert_infix_expression(
                    &if_exp.condition,
                    Literal::StringLiteral("x".to_string()),
                    "<",
                    Literal::StringLiteral("y".to_string()),
                );

                assert_eq!(if_exp.consequence.statements.len(), 1);

                if let Statement::ExpressionStatement(c) = &if_exp.consequence.statements[0] {
                    assert_identifier(c.expression.as_ref().unwrap(), "x");
                } else {
                    assert!(false);
                }

                assert!(if_exp.alternative.is_none());
            } else {
                assert!(false);
            }
        } else {
            assert!(false);
        }
    }

    #[test]
    fn test_function_literal_parsing() {
        let input = "fn (x, y) { x + y; }";
        let mut parser = Parser::new(Lexer::new(input));
        let program = parser.parse_program();
        if parser.errors.len() > 0 {
            println!("{:?}", parser.errors);
            assert!(false);
            return;
        }

        println!("{}", program);
        assert_eq!(program.statements.len(), 1);

        let exp_stmt = unwrap_to_expression_statement(&program.statements[0]).unwrap();
        let fn_exp = unwrap_to_fn_literal(exp_stmt.expression.as_ref().unwrap()).unwrap();
        assert_eq!(fn_exp.parameters.len(), 2);
        assert_eq!(fn_exp.parameters[0].token.to_string(), "x");
        assert_eq!(fn_exp.parameters[1].token.to_string(), "y");
        assert_eq!(fn_exp.body.statements.len(), 1);

        let body_stmt = unwrap_to_expression_statement(&fn_exp.body.statements[0])
            .unwrap()
            .expression
            .as_ref()
            .unwrap();

        assert_infix_expression(
            body_stmt,
            Literal::StringLiteral("x".to_string()),
            "+",
            Literal::StringLiteral("y".to_string()),
        );
    }

    #[test]
    fn test_call_expression() {
        let input = "add(1, 2 * 3, 4 + 5);";
        let mut parser = Parser::new(Lexer::new(input));
        let program = parser.parse_program();
        if parser.errors.len() > 0 {
            println!("{:?}", parser.errors);
            assert!(false);
            return;
        }

        assert_eq!(program.statements.len(), 1);

        let exp_stmt = unwrap_to_expression_statement(&program.statements[0]).unwrap();
        let call_exp = unwrap_to_call_expression(exp_stmt.expression.as_ref().unwrap()).unwrap();

        assert_identifier(&call_exp.function, "add");
        assert_eq!(call_exp.args.len(), 3);
        assert_literal_expression(&call_exp.args[0], Literal::NumberLiteral(1));
        assert_infix_expression(
            &call_exp.args[1],
            Literal::NumberLiteral(2),
            "*",
            Literal::NumberLiteral(3),
        );
        assert_infix_expression(
            &call_exp.args[2],
            Literal::NumberLiteral(4),
            "+",
            Literal::NumberLiteral(5),
        );
    }

    enum Literal {
        BoolLiteral(bool),
        NumberLiteral(i64),
        StringLiteral(String),
    }

    fn assert_prefix_expression(exp: &Expression, operator: &str, right: Literal) {
        if let Expression::PrefixExpression(op_exp) = exp {
            assert_eq!(op_exp.token.to_string(), operator);
            assert_literal_expression(&op_exp.right, right);
        } else {
            panic!("exp is not PrefixExpression");
        }
    }

    fn assert_infix_expression(exp: &Expression, left: Literal, operator: &str, right: Literal) {
        if let Expression::InfixExpression(op_exp) = exp {
            assert_literal_expression(&op_exp.left, left);
            assert_eq!(op_exp.token.to_string(), operator);
            assert_literal_expression(&op_exp.right, right);
        } else {
            panic!("exp is not InfixExpression");
        }
    }

    fn assert_literal_expression(exp: &Expression, value: Literal) {
        match value {
            Literal::BoolLiteral(v) => assert_bool_literal(exp, v),
            Literal::NumberLiteral(v) => assert_integer_literal(exp, v),
            Literal::StringLiteral(v) => assert_identifier(exp, &v),
        }
    }

    fn assert_integer_literal(exp: &Expression, value: i64) {
        if let Expression::IntegerLiteral(r) = exp {
            assert_eq!(r.value, value);
            assert_eq!(r.to_string(), value.to_string());
        } else {
            assert!(false);
        }
    }

    fn assert_bool_literal(exp: &Expression, value: bool) {
        if let Expression::BooleanLiteral(r) = exp {
            assert_eq!(r.value, value);
            assert_eq!(r.to_string(), value.to_string());
        } else {
            assert!(false);
        }
    }

    fn assert_identifier(exp: &Expression, value: &str) {
        if let Expression::Identifier(r) = exp {
            assert_eq!(r.to_string(), value);
        } else {
            assert!(false);
        }
    }

    fn assert_string_literal(exp: &Expression, value: &str) {
        if let Expression::StringLiteral(s) = exp {
            assert_eq!(s.to_string(), value);
        } else {
            assert!(false);
        }
    }

    fn unwrap_to_expression_statement(statement: &Statement) -> Option<&ExpressionStatement> {
        if let Statement::ExpressionStatement(e) = statement {
            Some(e)
        } else {
            None
        }
    }

    fn unwrap_to_fn_literal(expression: &Expression) -> Option<&FunctionLiteral> {
        if let Expression::FunctionLiteral(e) = expression {
            Some(e)
        } else {
            None
        }
    }

    fn unwrap_to_call_expression(expression: &Expression) -> Option<&CallExpression> {
        if let Expression::CallExpression(e) = expression {
            Some(e)
        } else {
            None
        }
    }

    #[test]
    fn test_string_literal_expression() {
        let input = "\"hello world!\"";
        let mut parser = Parser::new(Lexer::new(input));
        let program = parser.parse_program();

        let exp_stmt = unwrap_to_expression_statement(&program.statements[0]).unwrap();
        assert_string_literal(exp_stmt.expression.as_ref().unwrap(), "hello world!");
    }

    #[test]
    fn test_array_literal_expression() {
        let input = "[1, 2*2, 3+3]";
        let mut parser = Parser::new(Lexer::new(input));
        let program = parser.parse_program();

        let exp_stmt = unwrap_to_expression_statement(&program.statements[0]).unwrap();
        let expression = exp_stmt.expression.as_ref().unwrap();
        match expression {
            Expression::ArrayLiteral(a) => {
                assert_eq!(a.elements.len(), 3);
                assert_integer_literal(&a.elements[0], 1);
                assert_infix_expression(
                    &a.elements[1],
                    Literal::NumberLiteral(2),
                    "*",
                    Literal::NumberLiteral(2),
                );
                assert_infix_expression(
                    &a.elements[2],
                    Literal::NumberLiteral(3),
                    "+",
                    Literal::NumberLiteral(3),
                );
            }
            _ => panic!("not array literal"),
        }
    }

    #[test]
    fn test_hash_literal_string_key() {
        let mut expected = HashMap::new();
        expected.insert("one", 1);
        expected.insert("two", 2);
        expected.insert("three", 3);

        let input = r#"{"one": 1, "two": 2, "three": 3}"#;
        let mut parser = Parser::new(Lexer::new(input));
        let program = parser.parse_program();

        let exp_stmt = unwrap_to_expression_statement(&program.statements[0]).unwrap();
        let expression = exp_stmt.expression.as_ref().unwrap();
        match expression {
            Expression::HashLiteral(h) => {
                assert_eq!(h.paris.len(), 3);

                let mut iter = h.paris.iter();
                let kv = iter.next().unwrap();
                let v = expected.get(kv.0.to_string().as_str()).unwrap();
                assert_integer_literal(kv.1, *v);

                let kv = iter.next().unwrap();
                let v = expected.get(kv.0.to_string().as_str()).unwrap();
                assert_integer_literal(kv.1, *v);

                let kv = iter.next().unwrap();
                let v = expected.get(kv.0.to_string().as_str()).unwrap();
                assert_integer_literal(kv.1, *v);
            }
            _ => panic!("not hash literal"),
        }
    }

    #[test]
    fn test_hash_literal_empty() {
        let input = "{}";
        let mut parser = Parser::new(Lexer::new(input));
        let program = parser.parse_program();

        let exp_stmt = unwrap_to_expression_statement(&program.statements[0]).unwrap();
        let expression = exp_stmt.expression.as_ref().unwrap();
        match expression {
            Expression::HashLiteral(h) => {
                assert_eq!(h.paris.len(), 0);
            }
            _ => panic!("not hash literal"),
        }
    }

    #[test]
    fn test_hash_literal_with_expression() {
        let mut expected: HashMap<&str, fn(&Expression)> = HashMap::new();
        expected.insert("one", |e| {
            assert_infix_expression(e, Literal::NumberLiteral(1), "+", Literal::NumberLiteral(0))
        });

        expected.insert("two", |e| {
            assert_infix_expression(
                e,
                Literal::NumberLiteral(10),
                "-",
                Literal::NumberLiteral(8),
            );
        });

        expected.insert("three", |e| {
            assert_infix_expression(
                e,
                Literal::NumberLiteral(15),
                "/",
                Literal::NumberLiteral(5),
            )
        });

        let input = r#"{"one": 1 + 0, "two": 10 - 8, "three": 15 / 5}"#;
        let mut parser = Parser::new(Lexer::new(input));
        let program = parser.parse_program();

        let exp_stmt = unwrap_to_expression_statement(&program.statements[0]).unwrap();
        let expression = exp_stmt.expression.as_ref().unwrap();
        match expression {
            Expression::HashLiteral(h) => {
                assert_eq!(h.paris.len(), 3);
                let mut iter = h.paris.iter();
                let kv = iter.next().unwrap();
                let f = expected.get(kv.0.to_string().as_str()).unwrap();
                f(kv.1);

                let kv = iter.next().unwrap();
                let f = expected.get(kv.0.to_string().as_str()).unwrap();
                f(kv.1);

                let kv = iter.next().unwrap();
                let f = expected.get(kv.0.to_string().as_str()).unwrap();
                f(kv.1);
            }
            _ => panic!("not hash literal"),
        }
    }
}
