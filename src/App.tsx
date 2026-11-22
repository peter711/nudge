import { useCallback, useEffect, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import "./App.css";

type UiState = {
  task: string;
  interval_minutes: number;
};

function App() {
  const [task, setTask] = useState("");
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [pulse, setPulse] = useState(false);

  const pillRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const taskRef = useRef(task);
  const editingRef = useRef(editing);
  const closingRef = useRef(false);
  const dragRef = useRef<{ x: number; y: number; moved: boolean } | null>(null);
  const pulseTimer = useRef<number | undefined>(undefined);
  const pulseFrame = useRef<number | undefined>(undefined);

  taskRef.current = task;
  editingRef.current = editing;

  useEffect(() => {
    let disposed = false;
    const cleanups: UnlistenFn[] = [];

    const subscriptions = [
      listen<UiState>("task-changed", (event) => setTask(event.payload.task)),
      listen("edit-requested", () => {
        if (editingRef.current) return;
        setDraft(taskRef.current);
        setEditing(true);
      }),
      listen("nudge", () => {
        window.clearTimeout(pulseTimer.current);
        window.cancelAnimationFrame(pulseFrame.current ?? 0);
        setPulse(false);
        pulseFrame.current = window.requestAnimationFrame(() => {
          pulseFrame.current = window.requestAnimationFrame(() => setPulse(true));
        });
        pulseTimer.current = window.setTimeout(() => setPulse(false), 3800);
      }),
    ];

    Promise.all(subscriptions).then((fns) => {
      if (disposed) fns.forEach((fn) => fn());
      else cleanups.push(...fns);
    });

    invoke<UiState>("ui_state").then((state) => {
      if (!disposed) setTask(state.task);
    });

    return () => {
      disposed = true;
      window.clearTimeout(pulseTimer.current);
      window.cancelAnimationFrame(pulseFrame.current ?? 0);
      cleanups.forEach((fn) => fn());
    };
  }, []);

  // The window is exactly the size of the pill, so the invisible area never eats clicks.
  useEffect(() => {
    const pill = pillRef.current;
    if (!pill) return;
    const rect = pill.getBoundingClientRect();
    const width = Math.max(60, Math.ceil(rect.width));
    const height = Math.max(30, Math.ceil(rect.height));
    getCurrentWindow().setSize(new LogicalSize(width, height)).catch(() => {});
  }, [task, editing]);

  useEffect(() => {
    if (!editing) return;
    const input = inputRef.current;
    if (!input) return;
    input.focus();
    input.select();
  }, [editing]);

  const beginEdit = useCallback(() => {
    if (editingRef.current) return;
    invoke("begin_edit").catch(() => {});
  }, []);

  const finish = useCallback((value: string | null) => {
    if (closingRef.current) return;
    closingRef.current = true;
    setEditing(false);
    const done = value === null ? Promise.resolve() : invoke("set_task", { task: value });
    done.finally(() => {
      invoke("end_edit")
        .catch(() => {})
        .finally(() => {
          closingRef.current = false;
        });
    });
  }, []);

  const onPointerDown = (event: ReactPointerEvent) => {
    if (editing || event.button !== 0) return;
    dragRef.current = { x: event.clientX, y: event.clientY, moved: false };
  };

  const onPointerMove = (event: ReactPointerEvent) => {
    const drag = dragRef.current;
    if (!drag || drag.moved) return;
    if (Math.hypot(event.clientX - drag.x, event.clientY - drag.y) > 4) {
      drag.moved = true;
      dragRef.current = null;
      getCurrentWindow().startDragging().catch(() => {});
    }
  };

  const onPointerUp = () => {
    const drag = dragRef.current;
    dragRef.current = null;
    if (drag && !drag.moved) beginEdit();
  };

  return (
    <div
      ref={pillRef}
      className={[
        "pill",
        task ? "" : "empty",
        editing ? "editing" : "",
        pulse ? "nudge" : "",
      ]
        .filter(Boolean)
        .join(" ")}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={() => {
        dragRef.current = null;
      }}
    >
      {editing ? (
        <input
          ref={inputRef}
          value={draft}
          maxLength={120}
          spellCheck={false}
          autoComplete="off"
          placeholder="What are you working on?"
          onChange={(event) => setDraft(event.target.value)}
          onBlur={() => {
            if (editingRef.current) finish(draft);
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              finish(draft);
            } else if (event.key === "Escape") {
              event.preventDefault();
              finish(null);
            }
          }}
        />
      ) : (
        <>
          <span className="dot" aria-hidden="true" />
          <span className={task ? "label" : "label muted"}>{task || "No task"}</span>
        </>
      )}
    </div>
  );
}

export default App;
