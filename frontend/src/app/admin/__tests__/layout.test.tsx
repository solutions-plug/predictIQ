import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import AdminLayout from "../layout";

const ADMIN_KEY_STORAGE = "admin_api_key";

function mockSessionResponse(ok: boolean) {
  return jest.fn().mockResolvedValue({
    ok,
    status: ok ? 200 : 401,
    json: async () => ({ ok }),
  });
}

describe("AdminAuthGate", () => {
  beforeEach(() => {
    sessionStorage.clear();
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  it("renders the login form when no key is stored", () => {
    const fetchMock = mockSessionResponse(false);
    global.fetch = fetchMock as unknown as typeof fetch;

    render(
      <AdminLayout>
        <div>admin content</div>
      </AdminLayout>,
    );

    expect(screen.getByLabelText(/admin key/i)).toBeInTheDocument();
    expect(screen.queryByText("admin content")).not.toBeInTheDocument();
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("renders children when a valid stored key is present", async () => {
    sessionStorage.setItem(ADMIN_KEY_STORAGE, "valid-key");
    const fetchMock = mockSessionResponse(true);
    global.fetch = fetchMock as unknown as typeof fetch;

    render(
      <AdminLayout>
        <div>admin content</div>
      </AdminLayout>,
    );

    await waitFor(() => {
      expect(screen.getByText("admin content")).toBeInTheDocument();
    });

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/admin/session",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-admin-key": "valid-key" }),
      }),
    );
  });

  it("keeps the login form when the stored key is invalid", async () => {
    sessionStorage.setItem(ADMIN_KEY_STORAGE, "invalid-key");
    const fetchMock = mockSessionResponse(false);
    global.fetch = fetchMock as unknown as typeof fetch;

    render(
      <AdminLayout>
        <div>admin content</div>
      </AdminLayout>,
    );

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalled();
    });

    expect(screen.getByLabelText(/admin key/i)).toBeInTheDocument();
    expect(screen.queryByText("admin content")).not.toBeInTheDocument();
  });

  it("persists a submitted key to sessionStorage", async () => {
    const fetchMock = mockSessionResponse(true);
    global.fetch = fetchMock as unknown as typeof fetch;

    render(
      <AdminLayout>
        <div>admin content</div>
      </AdminLayout>,
    );

    await userEvent.type(screen.getByLabelText(/admin key/i), "new-key");
    await userEvent.click(screen.getByRole("button", { name: /sign in|submit|unlock/i }));

    await waitFor(() => {
      expect(sessionStorage.getItem(ADMIN_KEY_STORAGE)).toBe("new-key");
    });
  });
});
