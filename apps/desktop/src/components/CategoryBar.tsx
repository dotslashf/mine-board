import { useState } from "react";

import { useSoundboardStore } from "../stores/soundboardStore";

export function CategoryBar() {
  const categories = useSoundboardStore((s) => s.categories);
  const selectedCategoryId = useSoundboardStore((s) => s.selectedCategoryId);
  const selectCategory = useSoundboardStore((s) => s.selectCategory);
  const createCategory = useSoundboardStore((s) => s.createCategory);

  const [adding, setAdding] = useState(false);
  const [name, setName] = useState("");

  const handleAdd = () => {
    const trimmed = name.trim();
    if (trimmed) {
      void createCategory(trimmed);
    }
    setName("");
    setAdding(false);
  };

  return (
    <div className="flex flex-wrap items-center gap-2">
      <button
        type="button"
        onClick={() => selectCategory(null)}
        className={`rounded-md px-3 py-1.5 text-sm font-medium ${
          selectedCategoryId == null
            ? "bg-slate-700 text-white"
            : "bg-slate-800 text-slate-300 hover:bg-slate-700"
        }`}
      >
        All
      </button>
      {categories.map((category) => (
        <button
          key={category.id}
          type="button"
          onClick={() => selectCategory(category.id)}
          className={`rounded-md px-3 py-1.5 text-sm font-medium ${
            selectedCategoryId === category.id
              ? "bg-slate-700 text-white"
              : "bg-slate-800 text-slate-300 hover:bg-slate-700"
          }`}
        >
          {category.name}
        </button>
      ))}
      {adding ? (
        <form
          className="flex items-center gap-1"
          onSubmit={(event) => {
            event.preventDefault();
            handleAdd();
          }}
        >
          <input
            autoFocus
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="Category name"
            aria-label="New category name"
            className="rounded-md border border-slate-600 bg-slate-900 px-2 py-1.5 text-sm"
          />
          <button
            type="submit"
            className="rounded-md bg-slate-700 px-3 py-1.5 text-sm font-medium text-white"
          >
            Add
          </button>
          <button
            type="button"
            onClick={() => {
              setName("");
              setAdding(false);
            }}
            className="rounded-md px-2 py-1.5 text-sm text-slate-400 hover:text-white"
          >
            Cancel
          </button>
        </form>
      ) : (
        <button
          type="button"
          onClick={() => setAdding(true)}
          className="rounded-md border border-dashed border-slate-600 px-3 py-1.5 text-sm text-slate-400 hover:border-slate-400 hover:text-white"
        >
          + Category
        </button>
      )}
    </div>
  );
}
