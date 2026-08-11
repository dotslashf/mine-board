import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, mock, test } from "bun:test";

mock.module("../src/lib/api", () => apiFactory());
import { apiFactory, apiMock } from "./mocks/api";
import { SoundButton } from "../src/components/SoundButton";
import type { SoundClip } from "../src/lib/types";
import { useSoundboardStore } from "../src/stores/soundboardStore";

function clip(overrides: Partial<SoundClip> = {}): SoundClip {
  return {
    id: 1,
    name: "Cleared",
    file_path: "/tmp/cleared.wav",
    category_id: null,
    duration_ms: 5000,
    volume: 1.0,
    shortcut: "F1",
    enabled: true,
    sort_order: 0,
    ...overrides,
  };
}

beforeEach(() => {
  useSoundboardStore.setState({
    clips: [],
    categories: [],
    playing: {},
    playingStartedAt: {},
    selectedCategoryId: null,
    loading: false,
    error: null,
  });
  apiMock.playSound.mockClear();
  apiMock.stopSound.mockClear();
});

afterEach(() => {
  cleanup();
});

describe("SoundButton", () => {
  test("renders name, shortcut and duration", () => {
    render(<SoundButton clip={clip()} onEdit={mock()} />);

    expect(screen.getByRole("button", { name: /Cleared/i })).toBeDefined();
    expect(screen.getByText("F1")).toBeDefined();
    expect(screen.getByText("0:05")).toBeDefined();
  });

  test("clicking plays the clip", () => {
    render(<SoundButton clip={clip()} onEdit={mock()} />);

    fireEvent.click(screen.getByRole("button"));

    expect(apiMock.playSound).toHaveBeenCalledWith(1);
    expect(apiMock.stopSound).not.toHaveBeenCalled();
  });

  test("clicking while playing stops the clip", () => {
    useSoundboardStore.setState({
      playing: { 1: true },
      playingStartedAt: { 1: performance.now() },
    });
    render(<SoundButton clip={clip({ duration_ms: 0 })} onEdit={mock()} />);

    const button = screen.getByRole("button");
    expect(button.getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByText("Playing")).toBeDefined();

    fireEvent.click(button);

    expect(apiMock.stopSound).toHaveBeenCalledWith(1);
    expect(apiMock.playSound).not.toHaveBeenCalled();
  });

  test("playing with a duration shows the remaining time", () => {
    useSoundboardStore.setState({
      playing: { 1: true },
      playingStartedAt: { 1: performance.now() },
    });
    render(<SoundButton clip={clip()} onEdit={mock()} />);

    expect(screen.getByRole("status").textContent).toBe("0:05");
  });

  test("disabled clips render disabled and never play", () => {
    render(
      <SoundButton clip={clip({ enabled: false, name: "Offline" })} onEdit={mock()} />,
    );

    const button = screen.getByRole("button", { name: /Offline/i });
    expect(button.hasAttribute("disabled")).toBe(true);

    fireEvent.click(button);
    expect(apiMock.playSound).not.toHaveBeenCalled();
  });

  test("right-click opens edit", () => {
    const onEdit = mock();
    render(<SoundButton clip={clip()} onEdit={onEdit} />);

    fireEvent.contextMenu(screen.getByRole("button"));

    expect(onEdit).toHaveBeenCalledWith(clip());
  });
});
