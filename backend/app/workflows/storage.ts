import { Bucket } from "encore.dev/storage/objects";

export const evalArtifacts = new Bucket("eval-artifacts", {
  versioned: false,
});
