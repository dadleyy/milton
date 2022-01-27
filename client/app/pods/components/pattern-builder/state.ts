export type LedState = {
  ledn: number;
  hex: string;
};

export type Frame = {
  colors: Array<LedState>;
};

export type State = {
  frames: Array<Frame>;
};

export function empty(): State {
  return { frames: [] };
}

function emptyColorsForRange(range: [number, number]): Array<LedState> {
  const out = [];

  for (let i = range[0]; i <= range[1]; i++) {
    out.push({ hex: '#ff0000', ledn: i });
  }

  return out;
}

export function setColor(state: State, fi: number, led: LedState): State {
  const frames = state.frames.map((frame, index) => {
    const matches = fi === index;
    if (!matches) {
      return frame;
    }
    const colors = frame.colors.map(old => old.ledn === led.ledn ? led : old);
    return { ...frame, colors };
  });

  return {
    ...state,
    frames,
  };
}

export function addFrame(state: State, range: [number, number]): State {
  const colors = emptyColorsForRange(range);
  const frames = [...state.frames, { colors }];

  return {
    ...state,
    frames,
  };
}
