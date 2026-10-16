// SPDX-License-Identifier: GPL-3.0-or-later
import { createContext, useContext } from "react";

export interface People {
  /** The name to show for a sender: their contact name when Amber knows it, else the number or address. */
  nameFor: (identifier: string | null) => string;
  /** Whether this conversation has more than one other person (avatars and sender names show). */
  isGroup: boolean;
}

export const PeopleContext = createContext<People>({
  nameFor: (identifier) => identifier ?? "Unknown",
  isGroup: false,
});

export function usePeople(): People {
  return useContext(PeopleContext);
}
