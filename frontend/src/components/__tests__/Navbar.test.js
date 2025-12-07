import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import NavbarHarness from "./NavbarHarness.svelte";

const originalFetch = global.fetch;

afterEach(() => {
  global.fetch = originalFetch;
  vi.restoreAllMocks();
});

describe("Navbar", () => {
  it("shows login button when logged out and navigates to login", async () => {
    const navigateSpy = vi.fn();
    render(NavbarHarness, { props: { token: null, navigateSpy } });

    const loginButton = screen.getByRole("button", { name: /login/i });
    await fireEvent.click(loginButton);

    expect(navigateSpy).toHaveBeenCalledWith({
      path: "/login",
      replace: false,
    });
    expect(screen.queryByRole("button", { name: /logout/i })).not.toBeTruthy();
  });

  it("shows authenticated navigation when logged in", async () => {
    const navigateSpy = vi.fn();
    render(NavbarHarness, { props: { token: "token-123", navigateSpy } });

    const imagesButton = screen.getByRole("button", { name: /images/i });
    const logoutButton = screen.getByRole("button", { name: /logout/i });

    expect(screen.queryByRole("button", { name: /login/i })).toBeNull();

    await fireEvent.click(imagesButton);
    expect(navigateSpy).toHaveBeenCalledWith({
      path: "/images",
      replace: false,
    });
    expect(logoutButton).toBeInTheDocument();
  });

  it("logs out through the backend and removes the auth token", async () => {
    const navigateSpy = vi.fn();
    const removeAuthToken = vi.fn();
    const fetchMock = vi.fn(() =>
      Promise.resolve({ status: 200, text: () => Promise.resolve("ok") }),
    );
    global.fetch = fetchMock;

    render(NavbarHarness, {
      props: { token: "secret-token", navigateSpy, removeAuthToken },
    });

    await fireEvent.click(screen.getByRole("button", { name: /logout/i }));

    await waitFor(() => expect(removeAuthToken).toHaveBeenCalled());
    expect(fetchMock).toHaveBeenCalledWith("/backend/logout", {
      method: "POST",
      headers: {
        "Content-Type": "text/plain",
        Authorization: "Bearer secret-token",
      },
    });
  });
});
