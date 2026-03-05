import { and, asc, eq, inArray } from "drizzle-orm";

import { db } from "../core/database";
import { JsonValue, asOptionalJson } from "../core/json";
import { datapoints, queueItems } from "../core/schema";
import { newId } from "../core/utils";
import { QueueItem } from "./types";

function mapQueueItem(row: typeof queueItems.$inferSelect): QueueItem {
  return {
    id: row.id,
    dataset_id: row.datasetId,
    datapoint_id: row.datapointId,
    status: row.status as QueueItem["status"],
    claimed_by: row.claimedBy ?? undefined,
    claimed_at: row.claimedAt?.toISOString(),
    original_data: asOptionalJson(row.originalData),
    edited_data: asOptionalJson(row.editedData),
    created_at: row.createdAt.toISOString(),
    updated_at: row.updatedAt.toISOString(),
  };
}

export const QueueService = {
  async list(orgId: string, projectId: string, datasetId?: string): Promise<QueueItem[]> {
    const rows = await db
      .select()
      .from(queueItems)
      .where(
        datasetId
          ? and(
              eq(queueItems.orgId, orgId),
              eq(queueItems.projectId, projectId),
              eq(queueItems.datasetId, datasetId)
            )
          : and(eq(queueItems.orgId, orgId), eq(queueItems.projectId, projectId))
      )
      .orderBy(asc(queueItems.createdAt));

    return rows.map(mapQueueItem);
  },

  async enqueue(orgId: string, projectId: string, datasetId: string, datapointIds: string[]) {
    if (datapointIds.length === 0) return [];

    const points = await db
      .select({ id: datapoints.id, kind: datapoints.kind })
      .from(datapoints)
      .where(
        and(
          eq(datapoints.orgId, orgId),
          eq(datapoints.projectId, projectId),
          eq(datapoints.datasetId, datasetId),
          inArray(datapoints.id, datapointIds)
        )
      );

    const now = new Date();
    const inserted = await db
      .insert(queueItems)
      .values(
        points.map((p) => ({
          id: newId(),
          orgId,
          projectId,
          datasetId,
          datapointId: p.id,
          status: "pending",
          originalData: p.kind,
          createdAt: now,
          updatedAt: now,
        }))
      )
      .returning();

    return inserted.map(mapQueueItem);
  },

  async claim(orgId: string, projectId: string, id: string, claimedBy: string): Promise<QueueItem | null> {
    const [updated] = await db
      .update(queueItems)
      .set({
        status: "claimed",
        claimedBy,
        claimedAt: new Date(),
        updatedAt: new Date(),
      })
      .where(
        and(
          eq(queueItems.id, id),
          eq(queueItems.orgId, orgId),
          eq(queueItems.projectId, projectId),
          eq(queueItems.status, "pending")
        )
      )
      .returning();

    return updated ? mapQueueItem(updated) : null;
  },

  async submit(orgId: string, projectId: string, id: string, editedData: JsonValue): Promise<QueueItem | null> {
    const [updated] = await db
      .update(queueItems)
      .set({
        status: "completed",
        editedData,
        updatedAt: new Date(),
      })
      .where(
        and(
          eq(queueItems.id, id),
          eq(queueItems.orgId, orgId),
          eq(queueItems.projectId, projectId),
          eq(queueItems.status, "claimed")
        )
      )
      .returning();

    return updated ? mapQueueItem(updated) : null;
  },
};
