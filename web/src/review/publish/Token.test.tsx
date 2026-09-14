import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Token } from "./Token";

describe("the token field", () => {
  it("is masked before anyone asks for it, and says what it is without a placeholder", () => {
    // Found by its label at all is the association: `getByLabelText` only
    // reaches the box through the `for`/`id` pair a screen reader reads.
    render(<Token host="github.com" onToken={vi.fn()} />);

    expect(field().type).toBe("password");
    expect(screen.getByRole("button", { name: "Show token" })).toBeTruthy();
  });

  it("reveals and re-masks the same value, never a cleared one", () => {
    // The reason to reveal is to check a pasted token against its source. A
    // toggle that emptied the box would defeat the only thing it is for.
    render(<Token host="github.com" onToken={vi.fn()} />);
    fireEvent.change(field(), { target: { value: "github_pat_fake" } });

    fireEvent.click(screen.getByRole("button", { name: "Show token" }));
    expect(field().type).toBe("text");
    expect(field().value).toBe("github_pat_fake");

    fireEvent.click(screen.getByRole("button", { name: "Hide token" }));
    expect(field().type).toBe("password");
    expect(field().value).toBe("github_pat_fake");
  });

  it("puts the toggle on a real button, which is what makes it a keyboard control", () => {
    // Enter and Space come free from the element, and are lost the moment this
    // becomes a div with an onClick.
    render(<Token host="github.com" onToken={vi.fn()} />);
    const toggle = screen.getByRole("button", { name: "Show token" });

    toggle.focus();
    expect(document.activeElement).toBe(toggle);
    expect(toggle.tagName).toBe("BUTTON");
    expect(toggle.hasAttribute("disabled")).toBe(false);

    fireEvent.click(toggle);
    expect(screen.getByRole("button", { name: "Hide token" })).toBeTruthy();
  });

  it("sends what was typed, revealed or not", async () => {
    const onToken = vi.fn().mockResolvedValue(undefined);
    render(<Token host="enterprise.example" onToken={onToken} />);
    fireEvent.change(field(), { target: { value: "github_pat_fake" } });
    fireEvent.click(screen.getByRole("button", { name: "Show token" }));

    fireEvent.click(screen.getByRole("button", { name: "Save token for enterprise.example" }));

    await waitFor(() => expect(onToken).toHaveBeenCalledWith("enterprise.example", "github_pat_fake"));
  });

  it("goes back to empty and masked once the token is in", async () => {
    // Leaving it revealed would show the next token typed here from the first
    // keystroke, with nobody having asked for that.
    const onToken = vi.fn().mockResolvedValue(undefined);
    render(<Token host="github.com" onToken={onToken} />);
    fireEvent.change(field(), { target: { value: "github_pat_fake" } });
    fireEvent.click(screen.getByRole("button", { name: "Show token" }));

    fireEvent.click(screen.getByRole("button", { name: "Save token for github.com" }));

    await waitFor(() => expect(field().value).toBe(""));
    expect(field().type).toBe("password");
    expect(screen.getByRole("button", { name: "Show token" })).toBeTruthy();
  });

  it("keeps the value when the save is refused, so it can be tried again", async () => {
    const onToken = vi.fn().mockRejectedValue(new Error("The credential host changed."));
    render(<Token host="github.com" onToken={onToken} />);
    fireEvent.change(field(), { target: { value: "github_pat_fake" } });

    fireEvent.click(screen.getByRole("button", { name: "Save token for github.com" }));

    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("host changed"));
    expect(field().value).toBe("github_pat_fake");
    expect(field().type).toBe("password");
  });
});

const field = () => screen.getByLabelText("GitHub token") as HTMLInputElement;
