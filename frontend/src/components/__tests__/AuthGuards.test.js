import { render, screen, waitFor } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import AuthRequiredHarness from "./AuthRequiredHarness.svelte";
import UnauthRequiredHarness from "./UnauthRequiredHarness.svelte";

describe("AuthRequired", () => {
  it("renders children when an auth token is present", () => {
    const navigateSpy = vi.fn();
    render(AuthRequiredHarness, {
      props: { token: "token-123", navigateSpy, content: "Protected content" },
    });

    expect(screen.getByText("Protected content")).toBeInTheDocument();
    expect(navigateSpy).not.toHaveBeenCalled();
  });

  it("redirects to the fallback route when unauthenticated", async () => {
    const navigateSpy = vi.fn();
    render(AuthRequiredHarness, {
      props: {
        token: null,
        fallback: "/login",
        navigateSpy,
        content: "Protected content",
      },
    });

    await waitFor(() =>
      expect(navigateSpy).toHaveBeenCalledWith({
        path: "/login",
        replace: true,
      }),
    );
    expect(screen.queryByText("Protected content")).not.toBeInTheDocument();
  });
});

describe("UnauthRequired", () => {
  it("shows children when there is no auth token", () => {
    const navigateSpy = vi.fn();
    render(UnauthRequiredHarness, {
      props: { token: null, navigateSpy, content: "Login form" },
    });

    expect(screen.getByText("Login form")).toBeInTheDocument();
    expect(navigateSpy).not.toHaveBeenCalled();
  });

  it("redirects to fallback when already authenticated", async () => {
    const navigateSpy = vi.fn();
    render(UnauthRequiredHarness, {
      props: {
        token: "token-123",
        fallback: "/images",
        navigateSpy,
        content: "Login form",
      },
    });

    await waitFor(() =>
      expect(navigateSpy).toHaveBeenCalledWith({
        path: "/images",
        replace: true,
      }),
    );
    expect(screen.queryByText("Login form")).not.toBeInTheDocument();
  });
});
