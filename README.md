# README.md

## Basics
This application is expected to install, build, and run as specified. With correct input, it will write correctly formatted output to stdout. In the case of certain errors, it will write to stderr.

## Completeness
Generally, all specified inputs are expected to be handled correctly; extreme and unusual inputs may exceed the degree of testing that was applied.

## Correctness
* Unit and integration tests are included (these constitute most of the codebase).
* Newtypes and structs are used to guarantee correct typing of function inputs and outputs, and to make invalid states unrepresentable.

## Safety and Robustness
* There's no `unsafe` code.
* The type system is used to prevent errors at compile-time wherever possible.
* Error handling is minimalist: usually, if an unexpected state (like a mis-targeted dispute) is encountered, it is simply ignored, as recommended. In a production system, a reliable error-reporting channel would be a major design priority, and that may be the greatest difference between this code and something useful in production. 

## Efficiency
This code loads the input CSV as a string, sorts transactions by client, then calculates the resulting state of each client's transactions.

I was not given a specific time limit for this assignment, but given the rough amount of time I wanted to spend on it, I decided to focus on correctness and tests, rather than efficiency upgrades. If I were to continue improving efficiency:
* Functions could operate on iterators, rather than collections, and then
* The CSV parser could operate on a stream of lines, rather than an entire file string
* Once the transactions are sorted by client, the different sets have no further dependencies on each other and can be parallelized. Lines in the output can be in any order, so no special output reassembly would be required.

If multiple input streams were used, the above improvements would work as long as some mechanism guaranteed the chronological ordering of transactions as they arrived in the per-client sets.

## Maintainability
The actual operative code is clean and concise: most of this codebase is test code. Types and functions are named to minimize necessary comment explanations.

## Other Notes
In a production system, I would want to pay much more specific attention to certain risks:
* duplicate transaction IDs
* floating point arithmetic issues in balance changes (or possibly, use fixed-point or integral representation of balances)
* general arithmetic safety (overflow detection, etc)
* pending disputes that remain at the end of expected input

## Assumptions
* Available funds in an account can become negative if a withdrawal decreases available funds below the amount of a subsequent deposit dispute (i.e., 'overdrawing' disputes are presumed to be valid).
* Only deposits (not withdrawals) can be disputed.
* Accounts are opened by deposits: any transaction that precedes the first deposit on an account is ignored. If an account's activity does not include a deposit, the account will not appear in the program's output.

