import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, mock, test } from "bun:test";

mock.module("../src/lib/api", () => apiFactory());
import { apiFactory, apiMock } from "./mocks/api";
import { ClipDialog } from "../src/components/ClipDialog";
import { useSoundboardStore } from "../src/stores/soundboardStore";

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
  apiMock.importAudioFile.mockClear();
  apiMock.createSoundClip.mockClear();
});

afterEach(() => {
  cleanup();
});

describe("ClipDialog (create mode)", () => {
  test("empty name shows an error and does not import", async () => {
    const onClose = mock();
    render(
      <ClipDialog
        state={{ mode: "create", sourcePath: "/tmp/sound.wav", suggestedName: "Sound" }}
        onClose={onClose}
      />,
    );

    fireEvent.change(screen.getByLabelText("Name"), { target: { value: "" } });
    fireEvent.submit(screen.getByRole("dialog").querySelector("form")!);

    await waitFor(() => expect(screen.getByText("Name is required")).toBeDefined());
    expect(apiMock.importAudioFile).not.toHaveBeenCalled();
    expect(apiMock.createSoundClip).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });

  test("valid name imports the file and creates the clip", async () => {
    const onClose = mock();
    render(
      <ClipDialog
        state={{ mode: "create", sourcePath: "/tmp/sound.wav", suggestedName: "Sound" }}
        onClose={onClose}
      />,
    );

    fireEvent.change(screen.getByLabelText("Name"), { target: { value: "My clip" } });
    fireEvent.submit(screen.getByRole("dialog").querySelector("form")!);

    await waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(apiMock.importAudioFile).toHaveBeenCalledWith("/tmp/sound.wav");
    expect(apiMock.createSoundClip).toHaveBeenCalledWith(
      expect.objectContaining({
        name: "My clip",
        file_path: "/tmp/mocked-clip.wav",
        enabled: true,
      }),
    );
  });
});
