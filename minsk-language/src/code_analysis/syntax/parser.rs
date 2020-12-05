use unary_expression_syntax::UnaryExpressionSyntax;

use crate::code_analysis::diagnostic_bag::DiagnosticBag;

use super::{
    super::minsk_value::MinskValue, assignment_expression_syntax::AssignmentExpressionSyntax,
    name_expression_syntax::NameExpressionSyntax,
};

use super::{
    binary_expression_syntax::BinaryExpressionSyntax, expression_syntax::ExpressionSyntax,
    lexer::Lexer, literal_expression_syntax::LiteralExpressionSyntax,
    parenthesized_expression_syntax::ParenthesizedExpressionSyntax, syntax_facts::SyntaxFactsExt,
    syntax_kind::SyntaxKind, syntax_node::SyntaxNode, syntax_token::SyntaxToken,
    syntax_tree::SyntaxTree, unary_expression_syntax,
};

pub(super) struct Parser {
    tokens: Vec<SyntaxToken>,
    position: usize,
    diagnostics: DiagnosticBag,
}

impl Parser {
    pub(super) fn new(text: String) -> Self {
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

    pub(super) fn parse(mut self) -> SyntaxTree {
        let expression = self.parse_expression();
        let end_of_file_token = self.match_token(SyntaxKind::EndOfFile);

        SyntaxTree {
            root: SyntaxNode::ExpressionSyntax(expression),
            end_of_file_token,
            diagnostics: self.diagnostics(),
        }
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
            SyntaxKind::OpenParenthesis => {
                let open_parenthesis_token = self.next_token();
                let expression = self.parse_expression();
                let close_parenthesis_token = self.match_token(SyntaxKind::CloseParenthesis);
                ExpressionSyntax::Parenthesized(ParenthesizedExpressionSyntax {
                    open_parenthesis_token,
                    expression: Box::new(expression),
                    close_parenthesis_token,
                })
            }
            SyntaxKind::TrueKeyword | SyntaxKind::FalseKeyword => {
                let keyword_token = self.next_token();
                let value = Some(MinskValue::Boolean(
                    keyword_token.kind == SyntaxKind::TrueKeyword,
                ));
                ExpressionSyntax::Literal(LiteralExpressionSyntax {
                    literal_token: keyword_token,
                    value,
                })
            }
            SyntaxKind::Identifier => {
                let identifier_token = self.next_token();
                ExpressionSyntax::Name(NameExpressionSyntax { identifier_token })
            }
            _ => {
                let literal_token = self.match_token(SyntaxKind::Number);
                ExpressionSyntax::Literal(LiteralExpressionSyntax::new(literal_token))
            }
        }
    }

    pub(super) fn diagnostics(self) -> DiagnosticBag {
        self.diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binary_expression_honors_precedences_helper(op1: SyntaxKind, op2: SyntaxKind) {}

    #[test]
    fn binary_expression_honors_precedences() {}
}
