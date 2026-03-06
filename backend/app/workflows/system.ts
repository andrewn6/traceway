import type { CallOpts } from "encore.dev/api";

export const systemCallOpts: CallOpts = {
  authData: {
    userID: "traceway-daemon",
    principal: "traceway-daemon",
  } as never,
};
