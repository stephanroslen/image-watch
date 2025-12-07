import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import LoginHarness from "./LoginHarness.svelte";

const originalFetch = global.fetch;

afterEach(() => {
  global.fetch = originalFetch;
  vi.restoreAllMocks();
});

describe("Login page", () => {
  it("verifies the frontend hash on mount and focuses the username field", async () => {
    const verifyFrontendHash = vi.fn();
    render(LoginHarness, { props: { verifyFrontendHash } });

    await waitFor(() => expect(verifyFrontendHash).toHaveBeenCalled());
    expect(screen.getByPlaceholderText("Username")).toHaveFocus();
  });

  it("moves focus from username to password when pressing Enter", async () => {
    render(LoginHarness);

    const usernameInput = screen.getByPlaceholderText("Username");
    const passwordInput = screen.getByPlaceholderText("Password");

    usernameInput.focus();
    await fireEvent.keyDown(usernameInput, { key: "Enter", code: "Enter" });

    expect(passwordInput).toHaveFocus();
  });

  it("triggers login when pressing Enter in the password field", async () => {
    const setAuthToken = vi.fn();
    const fetchMock = vi.fn(() =>
      Promise.resolve({
        status: 200,
        text: () => Promise.resolve("new-token"),
      }),
    );
    global.fetch = fetchMock;

    render(LoginHarness, { props: { setAuthToken } });

    const usernameInput = screen.getByPlaceholderText("Username");
    const passwordInput = screen.getByPlaceholderText("Password");

    await fireEvent.input(usernameInput, { target: { value: "user" } });
    await fireEvent.input(passwordInput, { target: { value: "pass" } });

    passwordInput.focus();
    await fireEvent.keyDown(passwordInput, { key: "Enter", code: "Enter" });

    await waitFor(() =>
      expect(setAuthToken).toHaveBeenCalledWith("new-token"),
    );
  });

  it("does not set the auth token when the backend rejects the login", async () => {
    const setAuthToken = vi.fn();
    const fetchMock = vi.fn(() =>
      Promise.resolve({ status: 500, text: () => Promise.resolve("") }),
    );
    global.fetch = fetchMock;

    render(LoginHarness, { props: { setAuthToken } });

    const usernameInput = screen.getByPlaceholderText("Username");
    const passwordInput = screen.getByPlaceholderText("Password");
    const loginButton = screen.getByRole("button", { name: /login/i });

    await fireEvent.input(usernameInput, { target: { value: "user" } });
    await fireEvent.input(passwordInput, { target: { value: "bad" } });

    await fireEvent.click(loginButton);
    await waitFor(() => expect(fetchMock).toHaveBeenCalled());
    expect(setAuthToken).not.toHaveBeenCalled();
  });
});
