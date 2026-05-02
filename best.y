%{
/*
* Best - A statically typed Lisp, implemented in POSIX {lex(1), yacc(1)} and Rust.
* Copyright (C) 2026 Soumendra Ganguly
* 
* This program is free software: you can redistribute it and/or modify
* it under the terms of the GNU General Public License as published by
* the Free Software Foundation, either version 3 of the License, or
* (at your option) any later version.
* 
* This program is distributed in the hope that it will be useful,
* but WITHOUT ANY WARRANTY; without even the implied warranty of
* MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
* GNU General Public License for more details.
* 
* You should have received a copy of the GNU General Public License
* along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Location globals set by the lexer for each token. */
extern int g_tok_line;
extern int g_tok_col;

/* yyin is defined in lex.yy.c; we open the source file into it. */
extern FILE *yyin;

/* ---- Rust FFI: AST construction ---- */
extern void *best_make_int(long long val, int line, int col);
extern void *best_make_float(double val, int line, int col);
extern void *best_make_bool(int val, int line, int col);
extern void *best_make_str(const char *val, int line, int col);
extern void *best_make_id(const char *val, int line, int col);
extern void *best_make_type_node(const char *val, int line, int col);
extern void *best_make_list(void *exprs, int line, int col);
extern void *best_make_empty_list(int line, int col);
extern void *best_list_new(void *expr);
extern void *best_list_append(void *list, void *expr);
extern void *best_list_empty(void);
extern void  best_set_program(void *list);

int yylex(void);

void yyerror(const char *msg) {
    fprintf(stderr, "parse error at line %d, column %d: %s\n",
            g_tok_line, g_tok_col, msg);
    exit(1);
}

/* Called from Rust main to run the parser on a source file. */
int parse_file(const char *filename) {
    yyin = fopen(filename, "r");
    if (!yyin) {
        fprintf(stderr, "error: cannot open '%s': ", filename);
        perror("");
        return 1;
    }
    int rc = yyparse();
    fclose(yyin);
    return rc;
}
%}

%union {
    void       *node;
    long long   ival;
    double      fval;
    int         bval;
    char       *sval;
}

%token <ival> INT
%token <fval> FLOAT
%token <bval> BOOL
%token <sval> STR ID TYPE
%token        LPAREN RPAREN

%type <node> program expr_list expr list atom

%%

program
    : expr_list   { best_set_program($1); }
    |             { best_set_program(best_list_empty()); }
    ;

expr_list
    : expr              { $$ = best_list_new($1); }
    | expr_list expr    { $$ = best_list_append($1, $2); }
    ;

expr
    : atom   { $$ = $1; }
    | list   { $$ = $1; }
    ;

list
    : LPAREN expr_list RPAREN   { $$ = best_make_list($2, g_tok_line, g_tok_col); }
    | LPAREN RPAREN             { $$ = best_make_empty_list(g_tok_line, g_tok_col); }
    ;

atom
    : INT   { $$ = best_make_int($1, g_tok_line, g_tok_col); }
    | FLOAT { $$ = best_make_float($1, g_tok_line, g_tok_col); }
    | BOOL  { $$ = best_make_bool($1, g_tok_line, g_tok_col); }
    | STR   { $$ = best_make_str($1, g_tok_line, g_tok_col);  free($1); }
    | ID    { $$ = best_make_id($1, g_tok_line, g_tok_col);   free($1); }
    | TYPE  { $$ = best_make_type_node($1, g_tok_line, g_tok_col); free($1); }
    ;

%%
