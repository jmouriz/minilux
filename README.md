# minilux
### A small, multipurpose programming language

![minilux logo](minilux.png)

## About

Minilux is a minimal language designed for simplicity and learning. It features:

- **Variables** with dynamic typing (integers, strings, arrays, regex)
- **Control flow** (if/elseif/else, while loops)
- **Functions** (user-defined and built-in), including **arguments**
- **Arrays** with indexing and manipulation operations
- **String operations** including indexing and interpolation
- **Regular expressions** (literals, match operator, substitution)
- **TCP sockets** for network programming
- **Shell integration** for executing system commands

## Quick Start

### Building from Source

To build minilux from source, you need the **Rust toolchain** (cargo + rustc)

Debian/Ubuntu:
```sh
sudo apt install cargo
```

Fedora:
```sh
sudo dnf install rust cargo
```

macOS (homebrew):
```sh
brew install rust
```

Other systems:
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Build
```sh
make
```

### Install
```sh
make install
```

This installs `minilux` to `/usr/bin`, allowing you to run scripts directly with a shebang.

**mac users:** to build on macOS, edit the Makefile and change the path to `/usr/local/bin`

To uninstall:
```sh
make uninstall
```

### Rebuild (clean)
```sh
rm -rf target/
make uninstall; make clean; make
```

### Run an Example

```sh
./minilux examples/test.mi
```

Or make the script executable:

```sh
chmod +x examples/test.mi
./examples/test.mi
```

## Language Reference

### Variables

Variables start with `$` and can hold integers, strings, arrays (and regex values):

```minilux
$name = "Alexia"
$age = 42
$result = 1 + 2
$list = [1, 2, 3]
```

### Control Structures

#### if / elseif / else

**Important:** Compound boolean conditions require double parentheses:

```minilux
if (($age >= 18) AND ($name == "Alexia")) {
    printf("Adult named Alexia\n")
}
elseif ($age >= 13) {
    printf("Teenager\n")
}
else {
    printf("Child\n")
}
```

Simple conditions work with single parentheses:

```minilux
if ($age >= 18) {
    printf("Adult\n")
}
```

#### while loops

```minilux
$i = 1
while ($i <= 5) {
    printf("Count: ", $i, "\n")
    inc $i + 1
}
```

### Operators

#### Comparison Operators
- `==` equal
- `!=` not equal
- `>` greater than
- `<` less than
- `>=` greater than or equal
- `<=` less than or equal

#### Regex Match Operator
- `=~` matches a string against a regex

```minilux
$email = "test@example.com"
if ($email =~ /^[^@]+@[^@]+\.[^@]+$/) {
    printf("Valid email\n")
}
else {
    printf("Invalid email\n")
}
```

#### Logical Operators
- `AND` or `&&` logical and
- `OR` or `||` logical or

**Note:** When using AND/OR operators in conditions, use double parentheses:

```minilux
if (($x == 1) AND ($y == 2)) {
    printf("Both conditions met\n")
}

if (($a != 0) OR ($b != 0)) {
    printf("At least one is non-zero\n")
}
```

#### Arithmetic Operators
- `+` addition
- `-` subtraction
- `*` multiplication
- `/` division
- `%` modulo

Expressions support parentheses:

```minilux
$result = (10 + 5) * 2
$calc = 1 + (4 / 2)
```

### Arrays

```minilux
$arr = [1, 2, 3]
printf("Element 0: ", $arr[0], "\n")
printf("Length: ", len($arr), "\n")

push $arr, 4
pop $arr
shift $arr
unshift $arr, 0
```

String indexing also works:

```minilux
$text = "Hello"
printf("First char: ", $text[0], "\n")  # prints "H"
printf("Length: ", len($text), "\n")     # prints "5"
```

### Regular Expressions

#### Regex literal: `/.../`

Regex literals use the form `/pattern/`.

- Use `\/` to include a literal `/` inside the pattern.
- Keep escapes like `\s`, `\d`, `\.` as usual.

```minilux
$re = /foo[0-9]+/
```

#### Match with `=~`

```minilux
$text = "foo123"
if ($text =~ /foo[0-9]+/) {
    printf("matched!\n")
}
```

#### Substitution literal: `s/pat/repl/flags(expr)`

A callable substitution literal returns a **new string**:

- If there are **no matches**, it returns the input unchanged.
- Flags supported:
  - `g` = global replace (all matches)
  - `i` = case-insensitive
  - `m` = multi-line mode
  - `s` = dot matches newline

Replacement supports capture groups `$1..$n`.

```minilux
printf( s/o/O/g("foo"), "\n" )                 # fOO
printf( s/([0-9]+)/<$1>/g("a1b22"), "\n" )     # a<1>b<22
printf( s/\s+/ /g("hola   mundo"), "\n" )     # hola mundo
```

### Built-in Functions

#### printf() / print()

Print by concatenating all arguments:

```minilux
printf("Hello, ", $name, "!\n")
print("I am ", $age, " years old\n")
printf("Number: ", 42, "\n")
printf($name, " is ", $age, " years old\n")
```

Escape sequences:
- `\n` newline
- `\t` tab

#### read()

```minilux
printf("What is your name?")
read($name)
printf("Hello ", $name, "!\n")
```

#### len()

```minilux
$text = "Hello"
printf("Length: ", len($text), "\n")

$arr = [1, 2, 3]
printf("Array length: ", len($arr), "\n")
```

#### number()

```minilux
read($input)
$value = number($input)
printf("Twice is ", $value * 2, "\n")
```

#### lower() / upper()

```minilux
$answer = "YeS"
if (lower($answer) == "yes") {
    printf("Confirmed\n")
}
printf("Shouting: ", upper("minilux"), "\n")
```

#### shell()

```minilux
$user = shell("whoami")
printf("Current user: ", $user, "\n")
```

#### inc / dec

```minilux
$counter = 0
inc $counter + 1
inc $counter + 5
dec $counter - 2
```

#### Array Operations

- `push $array, value`
- `pop $array`
- `shift $array`
- `unshift $array, value`

#### Socket Operations

- `sockopen("name", "host", port)`
- `sockwrite("name", "data")`
- `sockread("name", $var)`
- `sockclose("name")`

### User-Defined Functions (with arguments)

Define functions with `function` and call them with parentheses:

```minilux
function hello($n) {
    printf("Hello ", $n, "!\n")
}

function add($a, $b) {
    return $a + $b
}

hello("Alexia")
$sum = add(2, 3)
printf("2 + 3 = ", $sum, "\n")
```

### Comments

Lines starting with `#` are comments (primarily for shebangs).

### File Extension

Minilux scripts use the `.mi` extension.

### Shebang Support

```minilux
#!/usr/bin/minilux

$name = "World"
printf("Hello, $name!\n")
```

## Examples

See the `examples/` directory for sample programs:
- `test.mi` - Basic language features
- `array_test_1.mi` - Array operations and string indexing
- `array_test_2.mi` - Array modification
- `http_test.mi` - TCP socket usage
- `regex_subst_demo.mi` - Regex match and substitution

## Project Structure

```
minilux/
├── src/
│   ├── main.rs         # Entry point and CLI
│   ├── value.rs        # Value type system
│   ├── lexer.rs        # Tokenization
│   ├── parser.rs       # AST generation
│   ├── interpreter.rs  # Execution engine
│   └── runtime.rs      # Runtime state management
├── examples/           # Example scripts
├── Cargo.toml          # Rust dependencies
├── Makefile            # Build automation
└── README.md           # This file
```

## License

This project is licensed under the **Mozilla Public License 2.0**. See the [LICENSE](LICENSE) file for details.
