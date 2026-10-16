// SPDX-License-Identifier: GPL-3.0-or-later
import { createContext, useContext } from "react";

export interface SearchState {
  /** What the user typed; empty when not searching. */
  query: string;
  /** Key of the message the find bar is currently on, so its highlight stands out. */
  activeKey: string | null;
}

export const SearchContext = createContext<SearchState>({ query: "", activeKey: null });

export function useSearch(): SearchState {
  return useContext(SearchContext);
}
