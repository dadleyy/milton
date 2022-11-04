type Flags = {
  apiRoot: string;
  uiRoot: string;
  loginURL: string;
  logoutURL: string;
  version: string;
};

declare const Elm: ElmRuntime<Flags>;

(function () {
  function metaValue(metaName: string): string | undefined {
    const container = document.querySelector(`meta[name="${metaName}"]`);
    return container ? container.getAttribute("content") ?? void 0 : void 0;
  }

  function boot(): void {
    const apiRoot = metaValue("apiRoot");
    const version = metaValue("version");
    const loginURL = metaValue("loginURL");
    const logoutURL = metaValue("logoutURL");
    const uiRoot = metaValue("uiRoot");

    if (!apiRoot || !version || !loginURL || !uiRoot || !logoutURL) {
      console.error("unable to create elm runtime environment");

      return void 0;
    }

    console.log("booting");
    const flags = { apiRoot, version, loginURL, uiRoot, logoutURL };
    Elm.Main.init({ flags });
  }

  window.addEventListener("DOMContentLoaded", boot);
})();
