## Ignored Variables

Variables containing only lowercase letters will be discarded from the
*Cut Register* upon a successful *Frame* take:
`${lowercase}` and not carry over into following frames.
This is meant to decrease noise in the *Cut Register* and
signal the relevance or lack of for a particular response.

* Lowercase variables can be combined with a leading underscore to ignore a
variable and hide it from any output: `${_hidden_and_ignored}` 
