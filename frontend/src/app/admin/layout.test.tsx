import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import AdminLayout from "./layout";

jest.mock("@/lib/env", () => ({
  resolveApiOrigin: () => "http://api.test",
}));

describe("AdminLayout auth gate", () => {
  beforeEach(() => {
    window.localStorage.clear();
    jest.restoreAllMocks();
  });

  it("unlocks the admin UI when the key is valid", async () => {
    const fetchMock = jest
      .spyOn(global, "fetch")
      .mockResolvedValue(new Response("{}", { status: 200 }));

    render(
      <AdminLayout>
        <div>admin content</div>
      </AdminLayout>,
    );

    await userEvent.type(screen.getByPlaceholderText("API key"), "good-key");
    await userEvent.click(screen.getByRole("button", { name: /unlock/i }));

    await waitFor(() => {
      expect(screen.getByText("admin content")).toBeInTheDocument();
    });
    expect(fetchMock).toHaveBeenCalledWith(
      "http://api.test/api/v1/admin/api-keys",
      expect.objectContaining({ method: "GET" }),
    );
  });

  it("keeps the gate closed and surfaces the reason when the key is invalid", async () => {
    jest
      .spyOn(global, "fetch")
      .mockResolvedValue(new Response("{}", { status: 401 }));

    render(
      <AdminLayout>
        <div>admin content</div>
      </AdminLayout>,
    );

    await userEvent.type(screen.getByPlaceholderText("API key"), "bad-key");
    await userEvent.click(screen.getByRole("button", { name: /unlock/i }));

    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent(/invalid api key/i);
    });
    expect(screen.queryByText("admin content")).not.toBeInTheDocument();
  });
});
