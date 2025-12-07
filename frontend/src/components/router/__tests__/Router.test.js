import { render, screen, waitFor } from "@testing-library/svelte";
import { vi } from "vitest";
import RouterHarness from "./RouterHarness.svelte";
import RouterDoubleDefaultHarness from "./RouterDoubleDefaultHarness.svelte";

describe("Router", () => {
  it("renders the matching route when the path exists", () => {
    render(RouterHarness, { props: { initialPath: "/login" } });

    expect(screen.getByText("Login page")).toBeInTheDocument();
  });

  it("navigates to the default route when the path is unknown", async () => {
    const navigateSpy = vi.fn();
    render(RouterHarness, {
      props: { initialPath: "/unknown", onNavigate: navigateSpy },
    });

    await waitFor(() =>
      expect(navigateSpy).toHaveBeenCalledWith({
        path: "/images",
        replace: true,
      }),
    );
    expect(navigateSpy).toHaveBeenCalledTimes(1);
    expect(screen.getByText("Images page")).toBeInTheDocument();
  });

  it("does not navigate when the current path matches a known route", () => {
    const navigateSpy = vi.fn();
    render(RouterHarness, {
      props: { initialPath: "/images", onNavigate: navigateSpy },
    });

    expect(screen.getByText("Images page")).toBeInTheDocument();
    expect(navigateSpy).not.toHaveBeenCalled();
  });

  it("throws when multiple default routes are registered", () => {
    expect(() => render(RouterDoubleDefaultHarness)).toThrow(
      "Default route already set",
    );
  });
});
