const png = (base64: string) => `data:image/png;base64,${base64}`;

export const CLASSIC_SLIDER_THUMB = png("iVBORw0KGgoAAAANSUhEUgAAABAAAAAQBAMAAADt3eJSAAAAAXNSR0IB2cksfwAAAAlwSFlzAAALEwAACxMBAJqcGAAAABVQTFRFAAAA7+/vzs7/nJz/Y3//6+v/Li5MdtgLlQAAADNJREFUeJxjZIACRkZFMM3HyGQIZvAiMxSA9AMQQxnIuEsygxnI+MuLxWRmRzDjDyPMGQA4AQyBPSg8HAAAAABJRU5ErkJggg==");

export const VOLUME_ICON_HIGH = png("iVBORw0KGgoAAAANSUhEUgAAABAAAAAQBAMAAADt3eJSAAAAJ1BMVEUAAADd3d0zM2ZmZsxVVVWZmf/MzP////9ERES7u7sAAAC5ubn19vbpfwX0AAAADXRSTlMA////////////////LQRBrQAAAGtJREFUeJxjYGBgEFJgAANGZQUWMEPI1IDBASwQZsDAAhZIN2gAMpSUw8rADNXI8jQDDqCi8PKyVAUQQzU4NBTMWGwMZaxabBqqAFLMvcsYwmDgXmyqADaQYZexAtgKhtOLF0AsZdi1AEgAAGP6FoncA7DYAAAAAElFTkSuQmCC");
export const VOLUME_ICON_MEDIUM = png("iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAMAAAAoLQ9TAAAAAXNSR0IB2cksfwAAAAlwSFlzAAALEwAACxMBAJqcGAAAADxQTFRFAAAA3d3dMzNmZmbMVVVVmZn/VVVVzMz/////REREREREu7u7VVVVVVVVVVVVAAAAREREubm59fb2VVVVQUCzFQAAABR0Uk5TAP///47/f////f//gIuE//r//5Tx/NXmAAAAiklEQVR4nEXP4QrCMAwE4EunA2U6p+//iAOnRUGb1Mva1f7KfSQlEaxPBFarmiUbdmkD5mAK9J8CnkMiuEjN8tXDe4VAYc5Jj68CHXpFBkcoQ5R9x9kMU3M4Pb3DOIYC54eMEahwufMTGYHoolY/xQRz+QMwuai1xYrwlrY6rtnisOA2t2vZg6VUP0lYRRHs4RZtAAAAAElFTkSuQmCC");
export const VOLUME_ICON_LOW = png("iVBORw0KGgoAAAANSUhEUgAAABAAAAAQBAMAAADt3eJSAAAAAXNSR0IB2cksfwAAAAlwSFlzAAALEwAACxMBAJqcGAAAACpQTFRFAAAA3d3dMzNmZmbMmZn/zMz/////REREREREu7u7AAAAREREubm59fb2uDWyVQAAAA50Uk5TAP////////3////6///GHgjWAAAAe0lEQVR4nGNkYGAQ4nvAwMAIREp/IQwhpT8XQAxGJaXXYIaQktIthwlAPqOSwumABgZGFVXZ97dv/GBgdFP4//7MCyBDRZ6R4Q6IsXiCIMOd118ZGFf9nChwJwGomGdfpiCYwcAzfeIbiBWnMt9CGHeO/00AMxhOXQMyAITlLREO3p2YAAAAAElFTkSuQmCC");
export const VOLUME_ICON_ZERO = png("iVBORw0KGgoAAAANSUhEUgAAABAAAAAQBAMAAADt3eJSAAAAAXNSR0IB2cksfwAAAAlwSFlzAAALEwAACxMBAJqcGAAAACFQTFRFAAAA3d3dPT09eXl5oqKizs7O////u7u7AAAAubm59fX1ScJAlwAAAAt0Uk5TAP////////////99dn3VAAAAbUlEQVR4nGNkYGAQ4nvAwMAIREp/IQwhpT8XQAxGJaXXYIaQktItEEOJUUnhNIihoiz//jaI4abw//0ZkC4VeUaGOyBG8wZBCKPj90YBMIOzf7YgmMHAWbXxDcSKGbPfQhgrb/1tADMYZrwAMgBvECkRX+/iMgAAAABJRU5ErkJggg==");

function iconForVolume(volume: number) {
  if (volume <= 0) return VOLUME_ICON_ZERO;
  if (volume <= 33) return VOLUME_ICON_LOW;
  if (volume <= 66) return VOLUME_ICON_MEDIUM;
  return VOLUME_ICON_HIGH;
}

function applyAssets(root: ParentNode = document) {
  root.querySelectorAll<HTMLInputElement>(".system-status-volume").forEach((slider) => {
    slider.style.setProperty("--classic-slider-thumb", `url(${CLASSIC_SLIDER_THUMB})`);
    const speaker = slider.parentElement?.querySelector<HTMLElement>(".system-status-speaker");
    const refresh = () => {
      if (speaker) speaker.style.backgroundImage = `url(${iconForVolume(Number(slider.value))})`;
    };
    if (slider.dataset.assetsInstalled !== "true") {
      slider.dataset.assetsInstalled = "true";
      slider.addEventListener("input", refresh);
    }
    refresh();
  });
}

const observer = new MutationObserver(() => applyAssets());
observer.observe(document.documentElement, { childList: true, subtree: true });
document.addEventListener("DOMContentLoaded", () => applyAssets());
applyAssets();
