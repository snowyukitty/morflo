import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import axe from "axe-core";
import { afterEach, describe, expect, test } from "vitest";

import { App } from "./App";
import { copy } from "./i18n/en";

function setView(query: string): void {
  window.history.replaceState({}, "", `/?${query}`);
}

afterEach(() => {
  cleanup();
  window.history.replaceState({}, "", "/");
  window.localStorage.clear();
});

describe("Morflo primary UI", () => {
  test("uses explicit calm labels for every engine trust source", () => {
    expect(copy.diagnostics.sourceLabels).toEqual({
      bundled: "Reviewed bundled engine",
      project: "Development engine",
      selected: "Session engine",
      system: "System engine",
      native: "Built-in image engine",
      missing: "Unavailable",
    });
  });

  test("shows the privacy promise and a single clear empty-state action", () => {
    setView("demo=empty&theme=light");
    render(<App />);

    expect(
      screen.getByRole("heading", { name: "Files in. Better formats out." }),
    ).toBeVisible();
    expect(screen.getByRole("button", { name: /choose files/i })).toBeVisible();
    expect(screen.getByText("Nothing leaves your device")).toBeVisible();
  });

  test("makes transparency loss explicit and responds to a safe format change", async () => {
    const user = userEvent.setup();
    setView("demo=queue&theme=light");
    render(<App />);

    expect(screen.getByText("JPEG has no transparency", { selector: "strong" })).toBeVisible();
    await user.click(screen.getByRole("button", { name: "PNG. Keeps transparency" }));
    expect(screen.queryByText("JPEG has no transparency", { selector: "strong" })).toBeNull();
    expect(screen.getByText("Keeps transparency")).toBeVisible();
  });

  test("supports queue removal and conversion from the keyboard", async () => {
    const user = userEvent.setup();
    setView("demo=queue&theme=light");
    render(<App />);

    const checkbox = screen.getByRole("checkbox", { name: "Select O'Reilly cup.jpg" });
    await user.click(checkbox);
    checkbox.blur();
    await user.keyboard("{Delete}");
    expect(screen.queryByText("O'Reilly cup.jpg")).toBeNull();

    await user.keyboard("{Control>}{Enter}{/Control}");
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Cancel all" })).toBeVisible(),
    );
  });

  test("recommends a usable JPEG output without an external engine", async () => {
    const user = userEvent.setup();
    setView("demo=engine-native&theme=light");
    render(<App />);
    await user.click(screen.getByText("O'Reilly cup.jpg"));
    expect(screen.getByRole("button", { name: "JPEG. Easy to share" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "WebP. Smaller web image" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: /^Smaller file/ }));
    expect(screen.getByRole("button", { name: "Smaller" })).toHaveAttribute(
      "class",
      expect.stringContaining("is-selected"),
    );
    expect(screen.getByLabelText("Resize mode")).toHaveValue("original");
    expect(screen.getByRole("button", { name: /^Keep transparency/ })).toBeDisabled();
  });

  test("applies sharing to each selected image without changing video settings", async () => {
    const user = userEvent.setup();
    setView("demo=queue&theme=light");
    render(<App />);
    await user.click(screen.getByRole("button", { name: "Select all" }));
    await user.click(screen.getByRole("button", { name: "Easy to share" }));
    expect(screen.getByLabelText("Resize mode")).toHaveValue("contain");
    expect(screen.getByLabelText(/^Width/)).toHaveValue(1920);
    await user.click(screen.getByText("O'Reilly cup.jpg"));
    expect(screen.getByRole("button", { name: "JPEG. Easy to share" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByLabelText(/^Height/)).toHaveValue(1920);
    await user.click(screen.getByText("京都散歩 🧳.mov"));
    expect(screen.getByRole("button", { name: /Universal MP4/ })).toHaveClass("is-selected");
  });

  test("keeps alpha when making a smaller image with the built-in engine", async () => {
    const user = userEvent.setup();
    setView("demo=engine-native&theme=dark");
    render(<App />);
    await user.click(screen.getByRole("button", { name: /^Smaller file/ }));
    expect(screen.getByRole("button", { name: "PNG. Keeps transparency" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByText("PNG · Keeps transparency · Size may grow")).toBeVisible();
    expect(screen.queryByText("JPEG has no transparency", { selector: "strong" })).toBeNull();
    expect(screen.getByLabelText("Resize mode")).toHaveValue("original");
  });

  test("has no serious automated accessibility violations in the working state", async () => {
    setView("demo=queue&theme=light");
    const { container } = render(<App />);
    const result = await axe.run(container, {
      rules: {
        "color-contrast": { enabled: false },
      },
    });

    expect(result.violations.filter((violation) => violation.impact === "serious")).toEqual([]);
  });

  test("cancels one active job without affecting the queued item", async () => {
    const user = userEvent.setup();
    setView("demo=active&theme=light");
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Cancel Tokyo morning.mov" }));
    expect(screen.getByText("Canceled")).toBeVisible();
    expect(screen.getByText("cover.jpg")).toBeVisible();
    expect(screen.getByText("Waiting")).toBeVisible();
  });

  test("offers a focused GIF range, tuned presets, and local preview action", async () => {
    const user = userEvent.setup();
    setView("demo=gif&theme=light");
    render(<App />);

    expect(screen.getByRole("heading", { name: "Choose the moment" })).toBeVisible();
    expect(screen.getByText("6.0 sec selected")).toBeVisible();
    expect(screen.getByText("0:00.0")).toBeVisible();
    expect(screen.getByText("0:06.0")).toBeVisible();
    expect(screen.getByText("0:12.4 source")).toBeVisible();
    const start = screen.getByRole("slider", { name: "GIF start time" });
    const end = screen.getByRole("slider", { name: "GIF end time" });
    expect(start).toHaveAttribute("max", "5.5");
    expect(start).toHaveAttribute("aria-valuetext", "0:00.0 into the source video");
    expect(end).toHaveAttribute("min", "0.5");
    fireEvent.change(start, { target: { value: "2" } });
    expect(end).toHaveAttribute("min", "2.5");
    expect(screen.getByText("4.0 sec selected")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "High quality" }));
    expect(screen.getByDisplayValue("720")).toBeInTheDocument();
    expect(screen.getByDisplayValue("18")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Play selected preview" }));
    expect(
      screen.getByText("Video preview is available in the Morflo desktop app"),
    ).toBeInTheDocument();
  });

  test("has no serious automated accessibility violations in the GIF workbench", async () => {
    setView("demo=gif&theme=dark");
    const { container } = render(<App />);
    const result = await axe.run(container, {
      rules: {
        "color-contrast": { enabled: false },
      },
    });

    expect(result.violations.filter((violation) => violation.impact === "serious")).toEqual([]);
  });

  test("keeps failed inspection out of conversion until retry succeeds", async () => {
    const user = userEvent.setup();
    setView("demo=error&theme=light");
    render(<App />);

    expect(screen.getByRole("button", { name: "Nothing ready" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Try again" }));
    expect(screen.getByRole("button", { name: "Convert" })).toBeEnabled();
  });

  test("restores settings focus and persists the versioned collision preference", async () => {
    const user = userEvent.setup();
    setView("demo=queue&theme=light");
    const first = render(<App />);

    await user.keyboard("{Control>},{/Control}");
    const close = screen.getByRole("button", { name: "Close settings" });
    expect(close).toHaveFocus();
    await user.selectOptions(screen.getByLabelText("Default collision behavior"), "skip");
    await user.keyboard("{Escape}");
    expect(screen.getByRole("button", { name: "Open settings" })).toHaveFocus();

    first.unmount();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "Open settings" }));
    expect(screen.getByLabelText("Default collision behavior")).toHaveValue("skip");
  });

  test("requires an explicit per-conversion choice before replacing an output", async () => {
    const user = userEvent.setup();
    setView("demo=queue&theme=light");
    render(<App />);

    await user.selectOptions(screen.getByLabelText("If a file exists"), "replace");
    expect(
      screen.getByText("This conversion may replace an existing output.", { exact: false }),
    ).toBeVisible();
  });

  test("turns a missing engine into a focused, actionable diagnostic flow", async () => {
    const user = userEvent.setup();
    setView("demo=engine-missing&theme=light");
    const { container } = render(<App />);

    const trigger = screen.getByRole("button", { name: "Open Diagnostics for details" });
    await user.click(trigger);

    expect(screen.getByRole("dialog", { name: "Local media engine" })).toBeVisible();
    expect(screen.getByRole("heading", { name: "Local media engine" })).toBeVisible();
    expect(screen.getByText("Engine needed")).toBeVisible();
    expect(screen.getAllByText("Unavailable")).toHaveLength(9);
    expect(screen.getByRole("button", { name: "Close engine diagnostics" })).toHaveFocus();

    const result = await axe.run(container, {
      rules: {
        "color-contrast": { enabled: false },
      },
    });
    expect(result.violations.filter((violation) => violation.impact === "serious")).toEqual([]);

    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog", { name: "Local media engine" })).toBeNull();
    expect(trigger).toHaveFocus();
  });

  test("shows capability evidence for a ready engine", () => {
    setView("demo=queue&panel=diagnostics&theme=dark");
    render(<App />);

    expect(screen.getByText("Ready to convert")).toBeVisible();
    expect(screen.getByText("8 / 8")).toBeVisible();
    expect(screen.getAllByText("Available")).toHaveLength(8);
    expect(screen.getAllByText("System engine").length).toBeGreaterThan(0);
  });

  test("turns a completed file into a truthful conversion receipt", () => {
    setView("demo=complete&theme=light");
    render(<App />);

    expect(screen.getByRole("heading", { name: "Your MP4 is ready" })).toBeVisible();
    expect(screen.getByText("61% smaller")).toBeVisible();
    expect(screen.getByText("1080 × 1920 · 0:42")).toBeVisible();
    expect(screen.getByText("京都散歩 🧳.mp4")).toBeVisible();
    expect(screen.getByText("Same folder as source")).toBeVisible();
    expect(screen.getByText("The source stayed untouched.")).toBeVisible();
    expect(screen.getByRole("button", { name: "Reveal in folder" })).toBeEnabled();
  });

  test("summarizes a selected completed batch without hiding partial failure", async () => {
    const user = userEvent.setup();
    setView("demo=partial&theme=light");
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Select all" }));
    expect(screen.getByRole("heading", { name: "1 of 2 outputs is ready" })).toBeVisible();
    expect(screen.getByText("1 WebP")).toBeVisible();
    expect(screen.getByText("1 needs attention")).toBeVisible();
    expect(screen.getByText("Every source stayed untouched.")).toBeVisible();
  });

  test("keeps the completed outcome accessible in dark mode", async () => {
    setView("demo=complete&theme=dark");
    const { container } = render(<App />);
    const result = await axe.run(container, {
      rules: {
        "color-contrast": { enabled: false },
      },
    });

    expect(result.violations.filter((violation) => violation.impact === "serious")).toEqual([]);
  });
});
