import type { ReadinessView } from "@/api";

/** What the panel says, per state: one line of why, then the steps as a list.
 *
 * Markdown, because somebody standing in front of a blocked button is looking
 * for what to do rather than reading, and a paragraph makes them find the steps
 * inside it while a list hands them over. The why still comes first: a step
 * nobody understands is a step done wrong. */
export const SAYS: Record<ReadinessView["state"], Said> = {
  ready: {
    title: "Ready to send",
    body: () => "The pull request is there and the token works.",
  },

  noRemote: {
    title: "This repository has no remote",
    body: () =>
      "There is nowhere to send a review to.\n\n" +
      "farol still reads the change here, and your comments still live under " +
      "the branch's own store. They just have no pull request to go to.",
  },

  noToken: {
    title: "farol needs a token of your own",
    body: () =>
      "The comments you left here go up as a review on this branch's pull " +
      "request, on the same lines and in the same words. To post them, farol " +
      "needs a token of your own.\n\n" +
      "- Create a **fine-grained personal access token** on GitHub.\n" +
      "- Give it **Pull requests: Read and write** on this repository.\n" +
      "- Paste it below.",
    token: true,
  },

  tokenRefused: {
    title: "GitHub would not take the token",
    body: () =>
      "One of two things:\n\n" +
      "- It expired.\n" +
      "- It does not carry **Pull requests: Read and write** on this " +
      "repository, which is what posting a review needs.\n\n" +
      "Create a new one and paste it below. It replaces the one farol has.",
    token: true,
  },

  branchNotPushed: {
    title: "The branch is not on GitHub yet",
    body: (branch) =>
      `A review is posted onto a pull request, and \`${branch}\` has none: ` +
      "GitHub has never seen the branch. Two steps, in this order:\n\n" +
      "- **Push it**, with the command below.\n" +
      "- **Open a pull request for it**, yourself on GitHub or through the " +
      "session that wrote the code.",
    command: (branch) => `git push -u origin ${branch}`,
    after: "Press the button below once both are done.",
    check: true,
  },

  noPullRequest: {
    title: "The branch has no pull request",
    body: (branch) =>
      `\`${branch}\` is on GitHub with nothing open on it, and a review is ` +
      "posted onto a pull request.\n\n" +
      "- Open one yourself, with the link below.\n" +
      "- Or ask the session that wrote the code. It knows what the change " +
      "was for, which is most of a description.",
    check: true,
  },
};

/** One state's worth of panel: the words, and which of the three fixed
 * pieces go under them. */
type Said = {
  title: string;
  body: (branch: string) => string;
  command?: (branch: string) => string;
  after?: string;
  token?: boolean;
  check?: boolean;
};
