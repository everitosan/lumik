/// <reference lib="webworker" />

export interface HistogramBins {
  r: Uint32Array;
  g: Uint32Array;
  b: Uint32Array;
}

self.onmessage = (e: MessageEvent<{ buffer: ArrayBuffer }>) => {
  const data = new Uint8ClampedArray(e.data.buffer);
  const r = new Uint32Array(256);
  const g = new Uint32Array(256);
  const b = new Uint32Array(256);

  for (let i = 0; i < data.length; i += 4) {
    r[data[i]]++;
    g[data[i + 1]]++;
    b[data[i + 2]]++;
  }

  self.postMessage({ r, g, b }, [r.buffer, g.buffer, b.buffer]);
};
