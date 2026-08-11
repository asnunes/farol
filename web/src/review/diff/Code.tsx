import type { Token } from "@/highlight/tokens";

/** The line, coloured if its grammar has arrived and plain until then.
 *
 * The palette for both themes rides on the token as custom properties, so the
 * stylesheet decides which one applies and nothing is tokenized twice. */
export function Code({ tokens, plain }: CodeProps) {
  if (!tokens) return <>{plain}</>;

  return (
    <>
      {tokens.map((token, i) => (
        <span key={i} className="tok" style={token.style}>
          {token.content}
        </span>
      ))}
    </>
  );
}

type CodeProps = { tokens?: Token[]; plain: string };
