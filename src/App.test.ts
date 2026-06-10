import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App.vue";
import { invoke } from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);

const loginConfig = {
  username: "01234567",
  password: "secret",
  operator: "@telecom",
};

function mountApp() {
  return mount(App, {
    global: {
      stubs: {
        LoginPage: {
          template:
            '<div><button data-test="manual-login" @click="$emit(\'login\', \'01234567\', \'bad\', \'@telecom\')">login form</button><button data-test="same-login" @click="$emit(\'login\', \'01234567\', \'secret\', \'@telecom\')">same login</button></div>',
        },
        StatusPage: {
          props: ["statusText", "isSuccess"],
          template:
            '<section data-test="status"><strong>{{ isSuccess ? "success" : "failure" }}</strong><p>{{ statusText }}</p></section>',
        },
      },
    },
  });
}

describe("App startup flow", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("shows the connected status when the connectivity check succeeds", async () => {
    invokeMock.mockResolvedValueOnce(true);

    const wrapper = mountApp();
    await flushPromises();

    expect(wrapper.text()).toContain("已连接到互联网");
    expect(invokeMock).toHaveBeenCalledWith("check_online");
    expect(invokeMock).toHaveBeenCalledTimes(1);
  });

  it("auto logs in when offline and saved config exists", async () => {
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "check_online") return false;
      if (command === "load_config") return loginConfig;
      if (command === "perform_login") return [true, "认证成功"];
      if (command === "save_config") return undefined;
      throw new Error(`unexpected command: ${command}`);
    });

    const wrapper = mountApp();
    await flushPromises();

    expect(invokeMock).toHaveBeenCalledWith("perform_login", loginConfig);
    expect(invokeMock).toHaveBeenCalledWith("save_config", {
      config: loginConfig,
    });
    expect(wrapper.text()).toContain("登录成功");
  });

  it("stays on the login page when offline and no config exists", async () => {
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "check_online") return false;
      if (command === "load_config") return null;
      throw new Error(`unexpected command: ${command}`);
    });

    const wrapper = mountApp();
    await flushPromises();

    expect(wrapper.text()).toContain("login form");
    expect(invokeMock).not.toHaveBeenCalledWith(
      "perform_login",
      expect.anything()
    );
  });

  it("shows a failure status when manual login fails", async () => {
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "check_online") return false;
      if (command === "load_config") return null;
      if (command === "perform_login") return [false, "密码错误"];
      throw new Error(`unexpected command: ${command}`);
    });

    const wrapper = mountApp();
    await flushPromises();
    await wrapper.get('[data-test="manual-login"]').trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("登录失败");
    expect(wrapper.text()).toContain("密码错误");
    expect(invokeMock).not.toHaveBeenCalledWith("save_config", expect.anything());
  });

  it("skips duplicate login when saved config is already online", async () => {
    let checkOnlineCalls = 0;
    let loadConfigCalls = 0;
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "check_online") return checkOnlineCalls++ > 0;
      if (command === "load_config") {
        return loadConfigCalls++ === 0 ? null : loginConfig;
      }
      throw new Error(`unexpected command: ${command}`);
    });

    const wrapper = mountApp();
    await flushPromises();
    await wrapper.get('[data-test="same-login"]').trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("无需重复登录");
    expect(invokeMock).not.toHaveBeenCalledWith(
      "perform_login",
      expect.anything()
    );
    expect(invokeMock).not.toHaveBeenCalledWith("save_config", expect.anything());
  });

  it("coalesces same saved auto login and manual login while request is in flight", async () => {
    let resolveLogin: (value: [boolean, string]) => void = () => {};
    const loginPromise = new Promise<[boolean, string]>((resolve) => {
      resolveLogin = resolve;
    });

    invokeMock.mockImplementation(async (command: string) => {
      if (command === "check_online") return false;
      if (command === "load_config") return loginConfig;
      if (command === "perform_login") return loginPromise;
      if (command === "save_config") return undefined;
      throw new Error(`unexpected command: ${command}`);
    });

    const wrapper = mountApp();
    await flushPromises();
    await wrapper.get('[data-test="same-login"]').trigger("click");
    await flushPromises();

    const loginCalls = invokeMock.mock.calls.filter(
      ([command]) => command === "perform_login"
    );
    expect(loginCalls).toHaveLength(1);

    resolveLogin([true, "认证成功"]);
    await flushPromises();

    expect(wrapper.text()).toContain("登录成功");
    expect(wrapper.text()).toContain("认证成功");
  });
});
