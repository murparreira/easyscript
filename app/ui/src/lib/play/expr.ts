// Dialogue Manager expressions (GDScript's), as far as playing a script needs: literals,
// variables, calls, `and` / `or` / `not`, comparisons and arithmetic. What the script
// cannot know (the game's own functions and variables) comes out as a question, the
// smallest part of the expression that decides it, for the writer to answer.

export type Value = string | number | boolean | null;

/** What playing knows: flags set, variables, and the writer's answers to questions. */
export type Known = {
  /** Is this flag set? */
  flag(name: string): boolean;
  /** Is this the name of a function that reads a flag (`has_flag`)? */
  readsFlag(name: string): boolean;
  /** A variable's value, when it was set while playing. */
  variable(path: string): Value | undefined;
  /** The writer's answer to a question, by its text. */
  answer(question: string): boolean | undefined;
};

/** A value, or the question that would decide it. */
export type Result = { value: Value } | { ask: string };

type Token = { kind: "num" | "str" | "id" | "op" | "punct"; text: string; start: number; end: number };

const OPS = ["==", "!=", "<=", ">=", "&&", "||", "+=", "-=", "*=", "/=", "<", ">", "!", "+", "-", "*", "/", "%", "="];

function tokenize(src: string): Token[] {
  const out: Token[] = [];
  let i = 0;
  while (i < src.length) {
    const c = src[i];
    if (/\s/.test(c)) {
      i++;
      continue;
    }
    const start = i;
    if (/[0-9]/.test(c)) {
      while (i < src.length && /[0-9._]/.test(src[i])) i++;
      out.push({ kind: "num", text: src.slice(start, i), start, end: i });
    } else if (c === '"' || c === "'") {
      i++;
      while (i < src.length && src[i] !== c) i += src[i] === "\\" ? 2 : 1;
      i++;
      out.push({ kind: "str", text: src.slice(start + 1, i - 1).replace(/\\(.)/g, "$1"), start, end: i });
    } else if (/[A-Za-z_]/.test(c)) {
      while (i < src.length && /[A-Za-z0-9_]/.test(src[i])) i++;
      out.push({ kind: "id", text: src.slice(start, i), start, end: i });
    } else {
      const op = OPS.find((o) => src.startsWith(o, i));
      if (op) {
        i += op.length;
        out.push({ kind: "op", text: op, start, end: i });
      } else {
        i++;
        out.push({ kind: "punct", text: c, start, end: i });
      }
    }
  }
  return out;
}

type Ast =
  | { t: "lit"; value: Value; start: number; end: number }
  | { t: "path"; path: string; start: number; end: number }
  | { t: "call"; path: string; args: Ast[]; start: number; end: number }
  | { t: "not"; arg: Ast; start: number; end: number }
  | { t: "neg"; arg: Ast; start: number; end: number }
  | { t: "bin"; op: string; left: Ast; right: Ast; start: number; end: number }
  | { t: "opaque"; start: number; end: number };

const BINDING: Record<string, number> = {
  or: 1,
  "||": 1,
  and: 2,
  "&&": 2,
  "==": 4,
  "!=": 4,
  "<": 4,
  ">": 4,
  "<=": 4,
  ">=": 4,
  in: 4,
  "+": 5,
  "-": 5,
  "*": 6,
  "/": 6,
  "%": 6,
};

class Parser {
  i = 0;
  constructor(
    readonly tokens: Token[],
    readonly src: string,
  ) {}

  peek(): Token | undefined {
    return this.tokens[this.i];
  }

  binaryOp(): string | null {
    const t = this.peek();
    if (!t) return null;
    if (t.kind === "op" && t.text in BINDING) return t.text;
    if (t.kind === "id" && (t.text === "and" || t.text === "or" || t.text === "in")) return t.text;
    return null;
  }

  expr(min = 0): Ast {
    let left = this.unary();
    for (let op = this.binaryOp(); op && BINDING[op] > min; op = this.binaryOp()) {
      this.i++;
      // `not in`
      const right = this.expr(BINDING[op]);
      left = { t: "bin", op, left, right, start: left.start, end: right.end };
    }
    return left;
  }

  unary(): Ast {
    const t = this.peek();
    if (!t) return { t: "opaque", start: this.src.length, end: this.src.length };
    if ((t.kind === "id" && t.text === "not") || (t.kind === "op" && t.text === "!")) {
      this.i++;
      const arg = this.expr(3);
      return { t: "not", arg, start: t.start, end: arg.end };
    }
    if (t.kind === "op" && t.text === "-") {
      this.i++;
      const arg = this.unary();
      return { t: "neg", arg, start: t.start, end: arg.end };
    }
    return this.primary();
  }

  primary(): Ast {
    const t = this.tokens[this.i++];
    if (!t) return { t: "opaque", start: this.src.length, end: this.src.length };
    if (t.kind === "num") return { t: "lit", value: Number(t.text.replace(/_/g, "")), start: t.start, end: t.end };
    if (t.kind === "str") return { t: "lit", value: t.text, start: t.start, end: t.end };
    if (t.kind === "punct" && t.text === "(") {
      const inner = this.expr();
      const close = this.peek();
      if (close?.text === ")") this.i++;
      return { ...inner, start: t.start, end: close?.end ?? inner.end };
    }
    if (t.kind === "id") {
      if (t.text === "true" || t.text === "false") return { t: "lit", value: t.text === "true", start: t.start, end: t.end };
      if (t.text === "null") return { t: "lit", value: null, start: t.start, end: t.end };
      let path = t.text;
      let end = t.end;
      while (this.peek()?.text === "." && this.tokens[this.i + 1]?.kind === "id") {
        path += `.${this.tokens[this.i + 1].text}`;
        end = this.tokens[this.i + 1].end;
        this.i += 2;
      }
      if (this.peek()?.text === "(") {
        this.i++;
        const args: Ast[] = [];
        while (this.peek() && this.peek()!.text !== ")") {
          args.push(this.expr());
          if (this.peek()?.text === ",") this.i++;
          else break;
        }
        const close = this.peek();
        if (close?.text === ")") this.i++;
        end = close?.end ?? end;
        return { t: "call", path, args, start: t.start, end };
      }
      return { t: "path", path, start: t.start, end };
    }
    return { t: "opaque", start: t.start, end: t.end };
  }
}

function parse(src: string): Ast | null {
  const tokens = tokenize(src);
  const parser = new Parser(tokens, src);
  const ast = parser.expr();
  // Something left over (an assignment, a lambda, syntax this does not know): opaque.
  return parser.i === tokens.length ? ast : null;
}

/** A question's text: the expression as written, its blanks evened out. */
export function question(text: string): string {
  return text.trim().replace(/\s+/g, " ");
}

const truthy = (v: Value) => v !== null && v !== false && v !== 0 && v !== "";

/** Evaluate an expression. A value, or the question that decides it. */
export function evaluate(src: string, known: Known): Result {
  const ast = parse(src);
  if (!ast) return asked(src, known);
  return run(ast, src, known);
}

/** The whole expression as one question, unless it was answered. */
function asked(text: string, known: Known): Result {
  const q = question(text);
  const answer = known.answer(q);
  return answer === undefined ? { ask: q } : { value: answer };
}

function run(ast: Ast, src: string, known: Known): Result {
  const text = src.slice(ast.start, ast.end);
  // An answered question decides, whatever it is.
  const answer = known.answer(question(text));
  if (answer !== undefined) return { value: answer };
  switch (ast.t) {
    case "lit":
      return { value: ast.value };
    case "path": {
      const value = known.variable(ast.path);
      return value === undefined ? { ask: question(text) } : { value };
    }
    case "call": {
      const name = ast.path.split(".").pop()!;
      const first = ast.args[0];
      if (known.readsFlag(name) && first?.t === "lit" && typeof first.value === "string") return { value: known.flag(first.value) };
      return { ask: question(text) };
    }
    case "not": {
      const arg = run(ast.arg, src, known);
      return "ask" in arg ? arg : { value: !truthy(arg.value) };
    }
    case "neg": {
      const arg = run(ast.arg, src, known);
      return "ask" in arg ? arg : { value: -Number(arg.value) };
    }
    case "bin": {
      const left = run(ast.left, src, known);
      if (ast.op === "and" || ast.op === "&&") {
        if ("value" in left && !truthy(left.value)) return { value: false };
        const right = run(ast.right, src, known);
        if ("ask" in left) return "value" in right && !truthy(right.value) ? { value: false } : left;
        return "ask" in right ? right : { value: truthy(right.value) };
      }
      if (ast.op === "or" || ast.op === "||") {
        if ("value" in left && truthy(left.value)) return { value: true };
        const right = run(ast.right, src, known);
        if ("ask" in left) return "value" in right && truthy(right.value) ? { value: true } : left;
        return "ask" in right ? right : { value: truthy(right.value) };
      }
      const right = run(ast.right, src, known);
      // A comparison with something unknown in it is asked whole: "is the level 0?".
      if ("ask" in left || "ask" in right) return { ask: question(text) };
      return { value: binary(ast.op, left.value, right.value) };
    }
    case "opaque":
      return { ask: question(text) };
  }
}

function binary(op: string, a: Value, b: Value): Value {
  switch (op) {
    case "==":
      return a === b;
    case "!=":
      return a !== b;
    case "<":
      return Number(a) < Number(b);
    case ">":
      return Number(a) > Number(b);
    case "<=":
      return Number(a) <= Number(b);
    case ">=":
      return Number(a) >= Number(b);
    case "+":
      return typeof a === "string" || typeof b === "string" ? `${a ?? ""}${b ?? ""}` : Number(a) + Number(b);
    case "-":
      return Number(a) - Number(b);
    case "*":
      return Number(a) * Number(b);
    case "/":
      return Number(b) === 0 ? null : Number(a) / Number(b);
    case "%":
      return Number(b) === 0 ? null : Number(a) % Number(b);
    case "in":
      return typeof b === "string" && typeof a === "string" ? b.includes(a) : false;
  }
  return null;
}

/** Is the condition true, false, or a question? */
export function holds(src: string, known: Known): boolean | { ask: string } {
  const result = evaluate(src, known);
  return "ask" in result ? result : truthy(result.value);
}

/** What an effect does to what playing knows: a flag set, a variable set. */
export type Change = { flag: string } | { variable: string; value: Value } | { unknown: string };

/** The change an effect (`do`, `set`) makes, as far as playing can tell. */
export function change(keyword: string, expr: string, known: Known & { setsFlag(name: string): boolean }): Change {
  const src = expr.trim();
  const tokens = tokenize(src);
  if (keyword === "set" || keyword.startsWith("$>")) {
    const at = tokens.findIndex((t) => t.kind === "op" && ["=", "+=", "-=", "*=", "/="].includes(t.text));
    if (at > 0 && tokens.slice(0, at).every((t) => t.kind === "id" || t.text === ".")) {
      const variable = src.slice(0, tokens[at].start).trim();
      const result = evaluate(src.slice(tokens[at].end), known);
      if ("ask" in result) return { unknown: src };
      const op = tokens[at].text;
      if (op === "=") return { variable, value: result.value };
      const before = known.variable(variable);
      if (before === undefined) return { unknown: src };
      return { variable, value: binary(op[0], before, result.value) };
    }
  }
  const ast = parse(src);
  if (ast?.t === "call") {
    const name = ast.path.split(".").pop()!;
    const first = ast.args[0];
    if (known.setsFlag(name) && first?.t === "lit" && typeof first.value === "string") return { flag: first.value };
  }
  return { unknown: src };
}

/** A value as the game would print it in a line (`{{x}}`). */
export function shown(value: Value): string {
  return value === null ? "" : String(value);
}
