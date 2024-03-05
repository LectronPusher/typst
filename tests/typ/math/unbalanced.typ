// Test unbalanced delimiters.
---
// this is an annoying behavior imo :/
$ 1/(2 (x) $
$ 1_(2 y (x) () $
$ 1/(2 y (x) (2(3)) $

// here's some more:
$ )( paren.l [a] sum paren.r) $
$ [pi([sum)] ) $
$ ( sqrt([sum) ] ) $
$ [ sqrt([infinity ) ] ) $