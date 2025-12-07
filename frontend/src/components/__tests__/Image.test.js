import { render, waitFor } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import ImageHarness from "./ImageHarness.svelte";

const originalFetch = global.fetch;
const originalIntersectionObserver = global.IntersectionObserver;
const originalCreateObjectURL = global.URL.createObjectURL;
const originalRevokeObjectURL = global.URL.revokeObjectURL;

let observeCallback;
const disconnectSpy = vi.fn();

class MockIntersectionObserver {
  constructor(callback) {
    observeCallback = callback;
  }

  observe() {}

  disconnect() {
    disconnectSpy();
  }
}

beforeEach(() => {
  observeCallback = undefined;
  disconnectSpy.mockClear();
  global.URL.createObjectURL = vi.fn();
  global.URL.revokeObjectURL = vi.fn();
});

afterEach(() => {
  global.fetch = originalFetch;
  global.IntersectionObserver = originalIntersectionObserver;
  global.URL.createObjectURL = originalCreateObjectURL || vi.fn();
  global.URL.revokeObjectURL = originalRevokeObjectURL || vi.fn();
  vi.restoreAllMocks();
});

describe("Image", () => {
  it("loads the image when it intersects and revokes the URL on destroy", async () => {
    const removeToken = vi.fn();
    const fetchMock = vi.fn(() =>
      Promise.resolve({
        blob: () => Promise.resolve(new Blob(["image-bytes"])),
      }),
    );

    const createdUrl = "blob:object-url";
    const createUrlSpy = vi.fn(() => createdUrl);
    const revokeUrlSpy = vi.fn();

    global.fetch = fetchMock;
    global.IntersectionObserver = MockIntersectionObserver;
    global.URL.createObjectURL = createUrlSpy;
    global.URL.revokeObjectURL = revokeUrlSpy;

    const { container, unmount } = render(ImageHarness, {
      props: {
        token: "token-123",
        removeToken,
        src: "/backend/data/photo.jpg",
        alt: "Photo",
      },
    });

    await waitFor(() => expect(observeCallback).toBeTypeOf("function"));
    expect(fetchMock).not.toHaveBeenCalled();
    observeCallback([{ isIntersecting: true }]);

    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith("/backend/data/photo.jpg", {
        headers: { Authorization: "Bearer token-123" },
      }),
    );
    await waitFor(() =>
      expect(container.querySelector("img").getAttribute("src")).toBe(
        createdUrl,
      ),
    );

    unmount();
    expect(disconnectSpy).toHaveBeenCalled();
    expect(revokeUrlSpy).toHaveBeenCalledWith(createdUrl);
  });

  it("removes the token when the backend responds with 401", async () => {
    const removeToken = vi.fn();
    global.IntersectionObserver = MockIntersectionObserver;
    global.fetch = vi.fn(() => Promise.reject({ status: 401 }));

    render(ImageHarness, {
      props: { token: "token-123", removeToken, src: "/backend/data/photo.jpg" },
    });

    await waitFor(() => expect(observeCallback).toBeTypeOf("function"));
    observeCallback([{ isIntersecting: true }]);

    await waitFor(() => expect(removeToken).toHaveBeenCalled());
  });

  it("uses eager loading by default and forwards an explicit lazy value", () => {
    global.IntersectionObserver = MockIntersectionObserver;

    const { container: defaultContainer, unmount: unmountDefault } = render(
      ImageHarness,
      { props: { src: "/backend/data/default.jpg" } },
    );
    expect(
      defaultContainer.querySelector("img").getAttribute("loading"),
    ).toBe("eager");
    unmountDefault();

    const { container: lazyContainer, unmount: unmountLazy } = render(
      ImageHarness,
      { props: { src: "/backend/data/lazy.jpg", loading: "lazy" } },
    );
    expect(lazyContainer.querySelector("img").getAttribute("loading")).toBe(
      "lazy",
    );
    unmountLazy();
  });
});
