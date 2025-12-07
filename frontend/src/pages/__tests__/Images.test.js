import { render, screen, waitFor } from "@testing-library/svelte";
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";
import ImagesHarness from "./ImagesHarness.svelte";

vi.mock("current-device", () => {
  const orientationListeners = [];
  const deviceMock = {
    type: "mobile",
    orientation: "portrait",
    onChangeOrientation: (cb) => orientationListeners.push(cb),
    _setOrientation: (orientation) => {
      deviceMock.orientation = orientation;
      orientationListeners.forEach((cb) => cb(orientation));
    },
  };
  return { default: deviceMock };
});

import device from "current-device";

const originalFetch = global.fetch;
const originalWebSocket = global.WebSocket;
const originalIntersectionObserver = global.IntersectionObserver;
const originalAnimate = Element.prototype.animate;

let sockets = [];

function createSocket() {
  const listeners = {
    open: [],
    close: [],
    error: [],
    message: [],
  };

  return {
    addEventListener: (type, cb) => listeners[type].push(cb),
    _emit: (type, event) => listeners[type].forEach((cb) => cb(event)),
  };
}

beforeEach(() => {
  sockets = [];
  global.WebSocket = vi.fn(() => {
    const socket = createSocket();
    sockets.push(socket);
    return socket;
  });

  global.IntersectionObserver = class {
    constructor() {}
    observe() {}
    disconnect() {}
  };

  Element.prototype.animate = () => {
    const animation = {
      onfinish: null,
      cancel: vi.fn(),
    };
    queueMicrotask(() => animation.onfinish && animation.onfinish());
    return animation;
  };
});

afterEach(() => {
  global.fetch = originalFetch;
  global.WebSocket = originalWebSocket;
  global.IntersectionObserver = originalIntersectionObserver;
  Element.prototype.animate = originalAnimate;
  vi.useRealTimers();
  vi.restoreAllMocks();
});

describe("Images page", () => {
  it("connects to the WebSocket and verifies the frontend hash on open", async () => {
    vi.useFakeTimers();
    const verifyFrontendHash = vi.fn();

    render(ImagesHarness, {
      props: { token: "abc", verifyFrontendHash },
    });

    expect(screen.getByText(/reconnecting/i)).toBeInTheDocument();

    sockets[0]._emit("open");

    await waitFor(() => expect(verifyFrontendHash).toHaveBeenCalled());
    expect(screen.queryByText(/reconnecting/i)).toBeNull();
    expect(global.WebSocket).toHaveBeenCalledWith(
      expect.stringContaining("/backend/ws"),
      ["bearer", "abc"],
    );
  });

  it("renders added images in timestamp order and removes items", async () => {
    render(ImagesHarness, { props: { token: "abc" } });

    sockets[0]._emit("message", {
      data: JSON.stringify({
        added: [
          ["newest.jpg", 3],
          ["older.jpg", 1],
        ],
      }),
    });

    await waitFor(() => {
      expect(screen.getAllByRole("img")).toHaveLength(2);
    });

    const orderedAlts = screen
      .getAllByRole("img")
      .map((img) => img.getAttribute("alt"));
    expect(orderedAlts).toEqual(["newest.jpg", "older.jpg"]);

    sockets[0]._emit("message", {
      data: JSON.stringify({ removed: ["newest.jpg"] }),
    });

    await waitFor(() =>
      expect(screen.getAllByRole("img").map((img) => img.alt)).toEqual([
        "older.jpg",
      ]),
    );
  });

  it("removes the auth token when the backend reports 401 on reconnect", async () => {
    vi.useFakeTimers();
    const removeAuthToken = vi.fn();
    global.fetch = vi.fn(() =>
      Promise.resolve({ status: 401, text: () => Promise.resolve("") }),
    );

    render(ImagesHarness, { props: { token: "abc", removeAuthToken } });

    sockets[0]._emit("close");

    await waitFor(() => expect(global.fetch).toHaveBeenCalled());
    await waitFor(() => expect(removeAuthToken).toHaveBeenCalled());
    vi.runOnlyPendingTimers();
    expect(global.WebSocket).toHaveBeenCalledTimes(1);
  });

  it("schedules reconnects with growing delays when the socket closes", async () => {
    vi.useFakeTimers();
    const setTimeoutSpy = vi.spyOn(global, "setTimeout");
    global.fetch = vi.fn(() =>
      Promise.resolve({ status: 500, text: () => Promise.resolve("") }),
    );

    render(ImagesHarness, { props: { token: "abc" } });

    sockets[0]._emit("close");
    await waitFor(() => expect(global.fetch).toHaveBeenCalled());
    await Promise.resolve();
    await Promise.resolve();

    expect(
      setTimeoutSpy.mock.calls.some(([, delay]) => delay === 500),
    ).toBe(true);

    vi.runOnlyPendingTimers();
    expect(global.WebSocket).toHaveBeenCalledTimes(2);

    sockets[1]._emit("error");
    await waitFor(() => expect(global.fetch).toHaveBeenCalledTimes(2));
    await Promise.resolve();
    await Promise.resolve();

    expect(
      setTimeoutSpy.mock.calls.some(([, delay]) => delay === 1000),
    ).toBe(true);
  });

  it("adjusts the grid columns when the device orientation changes", async () => {
    const { container } = render(ImagesHarness, { props: { token: "abc" } });
    const grid = container.querySelector(".grid");

    expect(grid.classList.contains("grid-cols-1")).toBe(true);

    device._setOrientation("landscape");
    await waitFor(() =>
      expect(grid.classList.contains("grid-cols-2")).toBe(true),
    );
    expect(grid.classList.contains("grid-cols-1")).toBe(false);
  });
});
