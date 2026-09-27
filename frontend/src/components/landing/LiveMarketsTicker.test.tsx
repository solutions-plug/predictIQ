import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import LiveMarketsTicker from "./LiveMarketsTicker";

const getFeaturedMarkets = vi.fn();

vi.mock("@/services/marketService", () => ({
  getFeaturedMarkets: (...args: unknown[]) => getFeaturedMarkets(...args),
}));

describe("LiveMarketsTicker", () => {
  beforeEach(() => {
    getFeaturedMarkets.mockReset();
  });

  it("disables the retry button and issues a single request on rapid double-click", async () => {
    let resolveRequest: (value: unknown) => void = () => {};
    getFeaturedMarkets.mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveRequest = resolve;
        }),
    );

    render(<LiveMarketsTicker />);

    const retryButton = await screen.findByRole("button", { name: /retry/i });

    const user = userEvent.setup();
    await user.click(retryButton);
    await user.click(retryButton);

    expect(getFeaturedMarkets).toHaveBeenCalledTimes(1);
    expect(retryButton).toBeDisabled();

    resolveRequest([]);

    await waitFor(() => expect(retryButton).not.toBeDisabled());
  });
});
