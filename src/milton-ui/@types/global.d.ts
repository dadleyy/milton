// esint-disable import/extensions

type ElmInitialization<T> = {
  node?: HTMLElement | null,
  flags?: T,
};

type ElmMain<T> = {
  init: (opts: ElmInitialization<T>) => void;
};

type ElmRuntime<T> = {
  Main: ElmMain<T>,
};
