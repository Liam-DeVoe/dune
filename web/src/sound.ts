// Sounds, as the C# client plays them: Browser.cs (PlaySound, StopSound, StopSounds,
// ChangeSoundVolume, CalculateVolume) and the functions they call in wwwroot/index.html. One
// Audio element per file is kept while it plays. Volumes are on a linear scale of 0 to 100.
//
// The volume settings (MusicComponent) are kept in local storage under the C# client's keys, as
// JSON numbers, and the effect and chat volumes are read from `soundSettings` by whatever plays
// effects (Client.CurrentEffectVolume, CurrentChatVolume).

import { BASE } from "./base";

const currentSounds = new Map<string, HTMLAudioElement>();

// Browser.CalculateVolume: a linear 0..100 to the exponential 0..1 an Audio element takes.
export function calculateVolume(linear: number): number {
  if (linear <= 0) return 0;
  if (linear >= 100) return 1;
  return Math.exp(0.05 * linear - 4) / Math.E;
}

const isValidSound = (sound: string) => sound !== "" && sound !== "?";

// The skin's URLs start with "./"; the app serves the art at its base path.
const resolve = (sound: string) => sound.replace(/^\.\//, BASE);

function fade(sound: HTMLAudioElement, fadeTime: number, done: () => void) {
  const step = sound.volume / 8;
  const interval = window.setInterval(() => {
    if (sound.volume <= step) {
      window.clearInterval(interval);
      done();
    } else {
      sound.volume -= step;
    }
  }, fadeTime / 8);
}

// Browser.PlaySound (PlaySound in index.html, without a fade-in: C# passes fadeTime 0).
export function playSound(file: string, volume = 100, loop = false) {
  if (!isValidSound(file)) return;
  let sound = currentSounds.get(file);
  if (sound) {
    sound.currentTime = 0;
  } else {
    sound = new Audio(resolve(file));
    sound.loop = loop;
    sound.volume = calculateVolume(volume);
    currentSounds.set(file, sound);
  }
  // Browsers refuse to play before the user has interacted with the page; C# ignores that too.
  void sound.play().catch(() => {});
}

// Browser.StopSound: fades out over three seconds.
export function stopSound(file: string) {
  if (!isValidSound(file)) return;
  const sound = currentSounds.get(file);
  if (!sound) return;
  fade(sound, 3000, () => {
    sound.pause();
    currentSounds.delete(file);
  });
}

// Browser.StopSounds: stops everything at once.
export function stopSounds() {
  currentSounds.forEach((sound) => sound.pause());
  currentSounds.clear();
}

// Browser.ChangeSoundVolume.
export function changeSoundVolume(file: string, volume: number) {
  if (!isValidSound(file)) return;
  const sound = currentSounds.get(file);
  if (sound) sound.volume = calculateVolume(volume);
}

const KEYS = { music: "treachery.online;setting.music", effect: "treachery.online;setting.effect", chat: "treachery.online;setting.chatvolume" } as const;
export type VolumeSetting = keyof typeof KEYS;

// MusicComponent.LoadSettings: a missing, unreadable or zero setting is 50.
function load(which: VolumeSetting): number {
  try {
    const v = Number(JSON.parse(localStorage.getItem(KEYS[which]) ?? "0"));
    return Number.isFinite(v) && v !== 0 ? v : 50;
  } catch {
    return 50;
  }
}

// The desired volumes, as MusicComponent and Client hold them.
export const soundSettings: Record<VolumeSetting, number> = { music: load("music"), effect: load("effect"), chat: load("chat") };

// MusicComponent.AdjustVolume: saves all three.
export function saveSoundSettings() {
  try {
    for (const k of Object.keys(KEYS) as VolumeSetting[]) localStorage.setItem(KEYS[k], JSON.stringify(soundSettings[k]));
  } catch {
    // Storage may be unavailable (private mode); the settings then last for this page.
  }
}
