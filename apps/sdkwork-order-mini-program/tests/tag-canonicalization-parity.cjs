// Parity check: the mini-program tag canonicalizer must agree with the Rust,
// TypeScript, and Dart implementations on the same cases.
const { normalizeOrderTag } = require("../src/services/order-service.js");

const cases = [
  ["VIP", "vip"],
  ["  vip  ", "vip"],
  ["summer sale", "summer-sale"],
  ["Risk__Review", "risk-review"],
  ["a--b", "a-b"],
  ["-lead", "lead"],
  ["trail-", "trail"],
  ["高价值", "高价值"],
  ["v2.0", "v2-0"],
];

const blanks = ["", "   ", "---", "___", " - "];
const rejected = ["vip!", "a/b", 'vip" OR 1=1'];

let failures = 0;

for (const [input, expected] of cases) {
  const got = normalizeOrderTag(input);
  if (got !== expected) {
    failures += 1;
    console.log(`MISMATCH ${JSON.stringify(input)}: got ${JSON.stringify(got)}, want ${JSON.stringify(expected)}`);
  }
}

for (const blank of blanks) {
  if (normalizeOrderTag(blank) !== "") {
    failures += 1;
    console.log(`BLANK NOT DROPPED ${JSON.stringify(blank)}`);
  }
}

for (const bad of rejected) {
  if (normalizeOrderTag(bad) !== "") {
    failures += 1;
    console.log(`INVALID NOT REJECTED ${JSON.stringify(bad)}`);
  }
}

// Idempotency: normalizing a canonical value must not change it.
for (const [, expected] of cases) {
  if (normalizeOrderTag(expected) !== expected) {
    failures += 1;
    console.log(`NOT IDEMPOTENT ${JSON.stringify(expected)}`);
  }
}

// Length ceiling, counted in code points like the server.
const atLimit = "x".repeat(32);
const overLimit = "x".repeat(33);
if (normalizeOrderTag(atLimit) !== atLimit) {
  failures += 1;
  console.log("AT-LIMIT TAG REJECTED");
}
if (normalizeOrderTag(overLimit) !== "") {
  failures += 1;
  console.log("OVER-LIMIT TAG ACCEPTED");
}

console.log(failures === 0 ? "mini-program canonicalization matches the other surfaces" : `${failures} failure(s)`);
process.exit(failures === 0 ? 0 : 1);
