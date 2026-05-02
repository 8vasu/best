%{
/*
* Best - A statically typed Lisp, implemented with POSIX lex/yacc and Rust.
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

#include "y.tab.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

/* Exported: set to the start position of the most-recently matched token. */
int g_tok_line = 1;
int g_tok_col  = 1;

/* Internal position tracking (one past the last consumed character). */
static int s_line = 1;
static int s_col  = 1;

/* Record where this token begins, then advance over yytext. */
static void begin_tok(void) {
    int i;
    g_tok_line = s_line;
    g_tok_col  = s_col;
    for (i = 0; yytext[i] != '\0'; i++) {
        if (yytext[i] == '\n') { s_line++; s_col = 1; }
        else { s_col++; }
    }
}

int yywrap(void) { return 1; }
%}

DIGIT   [0-9]
ID_CHAR [^ \t\n\r\(\)\;\"]

%%

[ \t\r]+              { begin_tok(); }
\n                    { begin_tok(); }
;[^\n]*               { begin_tok(); }

"("    { begin_tok(); return LPAREN; }
")"    { begin_tok(); return RPAREN; }

"true"  { begin_tok(); yylval.bval = 1; return BOOL; }
"false" { begin_tok(); yylval.bval = 0; return BOOL; }

"int"   { begin_tok(); yylval.sval = strdup(yytext); return TYPE; }
"float" { begin_tok(); yylval.sval = strdup(yytext); return TYPE; }
"bool"  { begin_tok(); yylval.sval = strdup(yytext); return TYPE; }
"str"   { begin_tok(); yylval.sval = strdup(yytext); return TYPE; }
"void"  { begin_tok(); yylval.sval = strdup(yytext); return TYPE; }

{DIGIT}+"."{DIGIT}+  { begin_tok(); yylval.fval = atof(yytext);  return FLOAT; }
{DIGIT}+              { begin_tok(); yylval.ival = atoll(yytext); return INT;   }

\"[^\"]*\"  { begin_tok(); yylval.sval = strdup(yytext); return STR; }

{ID_CHAR}+  { begin_tok(); yylval.sval = strdup(yytext); return ID; }

.  { begin_tok(); fprintf(stderr, "unexpected character '%s' at line %d\n", yytext, g_tok_line); exit(1); }

%%
