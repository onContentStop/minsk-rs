use unary_expression_syntax::UnaryExpressionSyntax;

use crate::code_analysis::{diagnostic_bag::DiagnosticBag, text::source_text::SourceText};

use super::{
    super::minsk_value::MinskValue,
    assignment_expression_syntax::AssignmentExpressionSyntax,
    block_statement_syntax::BlockStatementSyntax,
    compilation_unit::CompilationUnit,
    expression_statement_syntax::ExpressionStatementSyntax,
    for_statement_syntax::ForStatementSyntax,
    if_statement_syntax::{ElseClauseSyntax, IfStatementSyntax},
    name_expression_syntax::NameExpressionSyntax,
    statement_syntax::StatementSyntax,
    variable_declaration_syntax::VariableDeclarationSyntax,
    while_statement_syntax::WhileStatementSyntax,
};

use super::{
    binary_expression_syntax::BinaryExpressionSyntax, expression_syntax::ExpressionSyntax,
    lexer::Lexer, literal_expression_syntax::LiteralExpressionSyntax,
    parenthesized_expression_syntax::ParenthesizedExpressionSyntax, syntax_facts::SyntaxFactsExt,
    syntax_kind::SyntaxKind, syntax_token::SyntaxToken, unary_expression_syntax,
};

pub(super) struct Parser {
    tokens: Vec<SyntaxToken>,
    position: usize,
    diagnostics: DiagnosticBag,
}

impl Parser {
    pub(super) fn new(text: SourceText) -> Self {
        let mut lexer = Lexer::new(text);
        let mut tokens = vec![];
        loop {
            let token = lexer.next_token();
            let token_kind = token.kind;
            if token.kind != SyntaxKind::BadToken && token.kind != SyntaxKind::Whitespace {
                tokens.push(token);
            }
            if token_kind == SyntaxKind::EndOfFile {
                break;
            }
        }
        Self {
            tokens,
            position: 0,
            diagnostics: lexer.diagnostics(),
        }
    }

    fn peek(&self, offset: usize) -> &SyntaxToken {
        let index = self.position + offset;
        if index >= self.tokens.len() {
            self.tokens.last().unwrap()
        } else {
            self.tokens.get(index).unwrap()
        }
    }

    fn current(&self) -> SyntaxToken {
        self.peek(0).clone()
    }

    fn next_token(&mut self) -> SyntaxToken {
        let current = self.current();
        self.position += 1;
        current
    }

    fn match_token(&mut self, kind: SyntaxKind) -> SyntaxToken {
        if self.current().kind == kind {
            self.next_token()
        } else {
            self.diagnostics.report_unexpected_token(
                self.current().span,
                self.current().kind,
                kind,
            );
            SyntaxToken::new(kind, self.current().position, String::new(), None)
        }
    }

    pub fn parse_compilation_unit(&mut self) -> CompilationUnit {
        let statement = self.parse_statement();
        let end_of_file_token = self.match_token(SyntaxKind::EndOfFile);
        CompilationUnit::new(statement, end_of_file_token)
    }

    fn parse_statement(&mut self) -> StatementSyntax {
        match self.current().kind {
            SyntaxKind::OpenBrace => StatementSyntax::Block(self.parse_block_statement()),
            SyntaxKind::LetKeyword | SyntaxKind::VarKeyword => {
                StatementSyntax::VariableDeclaration(self.parse_variable_declaration())
            }
            SyntaxKind::ForKeyword => StatementSyntax::For(self.parse_for_statement()),
            SyntaxKind::IfKeyword => StatementSyntax::If(self.parse_if_statement()),
            SyntaxKind::WhileKeyword => StatementSyntax::While(self.parse_while_statement()),
            _ => StatementSyntax::Expression(self.parse_expression_statement()),
        }
    }

    fn parse_for_statement(&mut self) -> ForStatementSyntax {
        let keyword = self.match_token(SyntaxKind::ForKeyword);
        let identifier = self.match_token(SyntaxKind::Identifier);
        let equals_token = self.match_token(SyntaxKind::Equals);
        let lower_bound = self.parse_expression();
        let to_token = self.match_token(SyntaxKind::ToKeyword);
        let upper_bound = self.parse_expression();
        let body = self.parse_statement();
        ForStatementSyntax::new(
            keyword,
            identifier,
            equals_token,
            Box::new(lower_bound),
            to_token,
            Box::new(upper_bound),
            Box::new(body),
        )
    }

    fn parse_while_statement(&mut self) -> WhileStatementSyntax {
        let keyword = self.match_token(SyntaxKind::WhileKeyword);
        let condition = self.parse_expression();
        let body = self.parse_statement();
        WhileStatementSyntax::new(keyword, condition, Box::new(body))
    }

    fn parse_if_statement(&mut self) -> IfStatementSyntax {
        let keyword = self.match_token(SyntaxKind::IfKeyword);
        let condition = self.parse_expression();
        let statement = self.parse_statement();
        let else_clause = self.parse_optional_else_clause();
        IfStatementSyntax::new(keyword, condition, Box::new(statement), else_clause)
    }

    fn parse_optional_else_clause(&mut self) -> Option<ElseClauseSyntax> {
        if self.current().kind != SyntaxKind::ElseKeyword {
            return None;
        }

        let keyword = self.next_token();
        let statement = self.parse_statement();
        Some(ElseClauseSyntax::new(keyword, Box::new(statement)))
    }

    fn parse_variable_declaration(&mut self) -> VariableDeclarationSyntax {
        let expected = if self.current().kind == SyntaxKind::LetKeyword {
            SyntaxKind::LetKeyword
        } else {
            SyntaxKind::VarKeyword
        };
        let keyword = self.match_token(expected);
        let identifier = self.match_token(SyntaxKind::Identifier);
        let equals = self.match_token(SyntaxKind::Equals);
        let initializer = self.parse_expression();
        VariableDeclarationSyntax::new(keyword, identifier, equals, initializer)
    }

    fn parse_block_statement(&mut self) -> BlockStatementSyntax {
        let mut statements = Vec::<StatementSyntax>::new();
        let open_brace_token = self.match_token(SyntaxKind::OpenBrace);

        while self.current().kind != SyntaxKind::EndOfFile
            && self.current().kind != SyntaxKind::CloseBrace
        {
            let start_token = self.current();

            let statement = self.parse_statement();
            statements.push(statement);

            // if parse_statement didn't consume any tokens,
            // skip the current token and continue.
            // this avoids an infinite loop.
            //
            // do not need to report an error because
            // there's already an error trying to parse an expression statement
            if self.peek(0) == &start_token {
                self.next_token();
            }
        }

        let close_brace_token = self.match_token(SyntaxKind::CloseBrace);

        BlockStatementSyntax::new(open_brace_token, statements, close_brace_token)
    }

    fn parse_expression_statement(&mut self) -> ExpressionStatementSyntax {
        let expression = self.parse_expression();
        ExpressionStatementSyntax::new(expression)
    }

    fn parse_expression(&mut self) -> ExpressionSyntax {
        self.parse_assignment_expression()
    }

    fn parse_assignment_expression(&mut self) -> ExpressionSyntax {
        if self.peek(0).kind == SyntaxKind::Identifier && self.peek(1).kind == SyntaxKind::Equals {
            let identifier_token = self.next_token();
            let operator_token = self.next_token();
            let right = self.parse_assignment_expression();
            return ExpressionSyntax::Assignment(AssignmentExpressionSyntax {
                identifier_token,
                equals_token: operator_token,
                expression: Box::new(right),
            });
        }
        self.parse_binary_expression(0)
    }

    fn parse_binary_expression(&mut self, parent_precedence: usize) -> ExpressionSyntax {
        let unary_operator_precedence = self.current().kind.unary_operator_precedence();
        let mut left =
            if unary_operator_precedence != 0 && unary_operator_precedence >= parent_precedence {
                let operator_token = self.next_token();
                let operand = self.parse_binary_expression(unary_operator_precedence);
                ExpressionSyntax::Unary(UnaryExpressionSyntax {
                    operator_token,
                    operand: Box::new(operand),
                })
            } else {
                self.parse_primary_expression()
            };

        loop {
            let precedence = self.current().kind.binary_operator_precedence();
            if precedence == 0 || precedence <= parent_precedence {
                break;
            }

            let operator_token = self.next_token();
            let right = self.parse_binary_expression(precedence);
            left = ExpressionSyntax::Binary(BinaryExpressionSyntax {
                left: Box::new(left),
                operator_token,
                right: Box::new(right),
            });
        }

        left
    }

    fn parse_primary_expression(&mut self) -> ExpressionSyntax {
        match self.current().kind {
            SyntaxKind::OpenParenthesis => self.parse_parenthesized_expression(),
            SyntaxKind::TrueKeyword | SyntaxKind::FalseKeyword => self.parse_boolean_expression(),
            SyntaxKind::Number => self.parse_numeric_literal(),
            _ => self.parse_name_expression(),
        }
    }

    fn parse_parenthesized_expression(&mut self) -> ExpressionSyntax {
        let open_parenthesis_token = self.next_token();
        let expression = self.parse_expression();
        let close_parenthesis_token = self.match_token(SyntaxKind::CloseParenthesis);
        ExpressionSyntax::Parenthesized(ParenthesizedExpressionSyntax {
            open_parenthesis_token,
            expression: Box::new(expression),
            close_parenthesis_token,
        })
    }

    fn parse_boolean_expression(&mut self) -> ExpressionSyntax {
        let is_true = self.current().kind == SyntaxKind::TrueKeyword;
        let literal_token = if is_true {
            self.match_token(SyntaxKind::TrueKeyword)
        } else {
            self.match_token(SyntaxKind::FalseKeyword)
        };
        let value = Some(MinskValue::Boolean(is_true));
        ExpressionSyntax::Literal(LiteralExpressionSyntax {
            literal_token,
            value,
        })
    }

    fn parse_name_expression(&mut self) -> ExpressionSyntax {
        let identifier_token = self.match_token(SyntaxKind::Identifier);
        ExpressionSyntax::Name(NameExpressionSyntax { identifier_token })
    }

    fn parse_numeric_literal(&mut self) -> ExpressionSyntax {
        let literal_token = self.match_token(SyntaxKind::Number);
        ExpressionSyntax::Literal(LiteralExpressionSyntax::new(literal_token))
    }

    pub fn diagnostics(self) -> DiagnosticBag {
        self.diagnostics
    }
}

#[cfg(test)]
mod tests {
    use crate::code_analysis::syntax::{
        expression_statement_syntax::ExpressionStatementSyntax,
        name_expression_syntax::NameExpressionSyntax, statement_syntax::StatementSyntax,
        syntax_facts, syntax_tree::SyntaxTree,
    };

    use super::*;
    use itertools::Itertools;
    use spectral::prelude::*;
    use strum::IntoEnumIterator;
    use syntax_facts::{SyntaxFacts, SyntaxFactsExt};

    fn unary_expression_honors_precedences_helper(unary_kind: SyntaxKind, binary_kind: SyntaxKind) {
        let op1_precedence = unary_kind.unary_operator_precedence();
        let op2_precedence = binary_kind.binary_operator_precedence();

        let op1_text = SyntaxFacts::get_text(unary_kind).unwrap();
        let op2_text = SyntaxFacts::get_text(binary_kind).unwrap();
        let text = format!("{}a{}b", op1_text, op2_text);

        if op1_precedence >= op2_precedence {
            asserting!("syntax tree")
                .that(&SyntaxTree::parse(text).root().statement())
                .is_equal_to(&StatementSyntax::Expression(
                    ExpressionStatementSyntax::new(ExpressionSyntax::Binary(
                        BinaryExpressionSyntax {
                            left: Box::new(ExpressionSyntax::Unary(UnaryExpressionSyntax {
                                operator_token: SyntaxToken::new(
                                    unary_kind,
                                    0,
                                    String::from(op1_text),
                                    None,
                                ),
                                operand: Box::new(ExpressionSyntax::Name(NameExpressionSyntax {
                                    identifier_token: SyntaxToken::new(
                                        SyntaxKind::Identifier,
                                        1,
                                        String::from("a"),
                                        None,
                                    ),
                                })),
                            })),
                            operator_token: SyntaxToken::new(
                                binary_kind,
                                2,
                                String::from(op2_text),
                                None,
                            ),
                            right: Box::new(ExpressionSyntax::Name(NameExpressionSyntax {
                                identifier_token: SyntaxToken::new(
                                    SyntaxKind::Identifier,
                                    2 + op2_text.len(),
                                    String::from("b"),
                                    None,
                                ),
                            })),
                        },
                    )),
                ));
        } else {
            asserting!("syntax tree")
                .that(&SyntaxTree::parse(text).root().statement())
                .is_equal_to(&StatementSyntax::Expression(
                    ExpressionStatementSyntax::new(ExpressionSyntax::Unary(
                        UnaryExpressionSyntax {
                            operator_token: SyntaxToken::new(
                                unary_kind,
                                0,
                                String::from(op1_text),
                                None,
                            ),
                            operand: Box::new(ExpressionSyntax::Binary(BinaryExpressionSyntax {
                                left: Box::new(ExpressionSyntax::Name(NameExpressionSyntax {
                                    identifier_token: SyntaxToken::new(
                                        SyntaxKind::Identifier,
                                        1,
                                        String::from("a"),
                                        None,
                                    ),
                                })),
                                operator_token: SyntaxToken::new(
                                    binary_kind,
                                    2,
                                    String::from(op2_text),
                                    None,
                                ),
                                right: Box::new(ExpressionSyntax::Name(NameExpressionSyntax {
                                    identifier_token: SyntaxToken::new(
                                        SyntaxKind::Identifier,
                                        2 + op2_text.len(),
                                        String::from("b"),
                                        None,
                                    ),
                                })),
                            })),
                        },
                    )),
                ));
        }
    }

    fn binary_expression_honors_precedences_helper(op1: SyntaxKind, op2: SyntaxKind) {
        let op1_precedence = op1.binary_operator_precedence();
        let op2_precedence = op2.binary_operator_precedence();

        let op1_text = SyntaxFacts::get_text(op1).unwrap();
        let op2_text = SyntaxFacts::get_text(op2).unwrap();
        let text = format!("a{}b{}c", op1_text, op2_text);

        if op1_precedence >= op2_precedence {
            asserting!("syntax tree")
                .that(&SyntaxTree::parse(text).root().statement())
                .is_equal_to(&StatementSyntax::Expression(
                    ExpressionStatementSyntax::new(ExpressionSyntax::Binary(
                        BinaryExpressionSyntax {
                            left: Box::new(ExpressionSyntax::Binary(BinaryExpressionSyntax {
                                left: Box::new(ExpressionSyntax::Name(NameExpressionSyntax {
                                    identifier_token: SyntaxToken::new(
                                        SyntaxKind::Identifier,
                                        0,
                                        String::from("a"),
                                        None,
                                    ),
                                })),
                                operator_token: SyntaxToken::new(
                                    op1,
                                    1,
                                    String::from(op1_text),
                                    None,
                                ),
                                right: Box::new(ExpressionSyntax::Name(NameExpressionSyntax {
                                    identifier_token: SyntaxToken::new(
                                        SyntaxKind::Identifier,
                                        1 + op1_text.len(),
                                        String::from("b"),
                                        None,
                                    ),
                                })),
                            })),
                            operator_token: SyntaxToken::new(
                                op2,
                                2 + op1_text.len(),
                                String::from(op2_text),
                                None,
                            ),
                            right: Box::new(ExpressionSyntax::Name(NameExpressionSyntax {
                                identifier_token: SyntaxToken::new(
                                    SyntaxKind::Identifier,
                                    2 + op1_text.len() + op2_text.len(),
                                    String::from("c"),
                                    None,
                                ),
                            })),
                        },
                    )),
                ));
        } else {
            asserting!("syntax tree")
                .that(&SyntaxTree::parse(text).root().statement())
                .is_equal_to(&StatementSyntax::Expression(
                    ExpressionStatementSyntax::new(ExpressionSyntax::Binary(
                        BinaryExpressionSyntax {
                            left: Box::new(ExpressionSyntax::Name(NameExpressionSyntax {
                                identifier_token: SyntaxToken::new(
                                    SyntaxKind::Identifier,
                                    0,
                                    String::from("a"),
                                    None,
                                ),
                            })),
                            operator_token: SyntaxToken::new(op1, 1, String::from(op1_text), None),
                            right: Box::new(ExpressionSyntax::Binary(BinaryExpressionSyntax {
                                left: Box::new(ExpressionSyntax::Name(NameExpressionSyntax {
                                    identifier_token: SyntaxToken::new(
                                        SyntaxKind::Identifier,
                                        1 + op1_text.len(),
                                        String::from("b"),
                                        None,
                                    ),
                                })),
                                operator_token: SyntaxToken::new(
                                    op2,
                                    2 + op1_text.len(),
                                    String::from(op2_text),
                                    None,
                                ),
                                right: Box::new(ExpressionSyntax::Name(NameExpressionSyntax {
                                    identifier_token: SyntaxToken::new(
                                        SyntaxKind::Identifier,
                                        2 + op1_text.len() + op2_text.len(),
                                        String::from("c"),
                                        None,
                                    ),
                                })),
                            })),
                        },
                    )),
                ));
        }
    }

    #[test]
    fn binary_expression_honors_precedences() {
        for (op1, op2) in get_binary_operator_pairs() {
            binary_expression_honors_precedences_helper(op1, op2);
        }
    }

    #[test]
    fn unary_expression_honors_precedences() {
        for (unary, binary) in get_unary_operator_pairs() {
            unary_expression_honors_precedences_helper(unary, binary);
        }
    }

    fn get_unary_operators() -> Vec<SyntaxKind> {
        SyntaxKind::iter()
            .filter(|k| k.unary_operator_precedence() > 0)
            .collect()
    }

    fn get_binary_operators() -> Vec<SyntaxKind> {
        SyntaxKind::iter()
            .filter(|k| k.binary_operator_precedence() > 0)
            .collect()
    }

    fn get_binary_operator_pairs() -> Vec<(SyntaxKind, SyntaxKind)> {
        get_binary_operators()
            .iter()
            .cloned()
            .cartesian_product(get_binary_operators())
            .collect()
    }

    fn get_unary_operator_pairs() -> Vec<(SyntaxKind, SyntaxKind)> {
        get_unary_operators()
            .iter()
            .cloned()
            .cartesian_product(get_binary_operators())
            .collect()
    }
}
