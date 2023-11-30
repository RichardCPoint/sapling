/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import type {ApplyPreviewsFuncType, PreviewContext} from '../previews';
import type {CommitInfo} from '../types';

import {latestSuccessor} from '../SuccessionTracker';
import {CommitPreview} from '../previews';
import {exactRevset} from '../types';
import {firstLine} from '../utils';
import {Operation} from './Operation';

/**
 * Returns [bottom, top] of an array.
 */
function ends<T>(range: Array<T>): [T, T] {
  return [range[0], range[range.length - 1]];
}

export class FoldOperation extends Operation {
  constructor(private foldRange: Array<CommitInfo>, newMessage: string) {
    super('FoldOperation');
    this.newTitle = firstLine(newMessage);
    this.newDescription = newMessage.substring(firstLine(newMessage).length + 1);
  }
  private newTitle: string;
  private newDescription: string;

  static opName = 'Fold';

  getArgs() {
    const [bottom, top] = ends(this.foldRange);
    return [
      'fold',
      '--exact',
      exactRevset(`${bottom.hash}::${top.hash}`),
      '--message',
      `${this.newTitle}\n${this.newDescription}`,
    ];
  }

  public getFoldRange(): Array<CommitInfo> {
    return this.foldRange;
  }
  public getFoldedMessage(): [string, string] {
    return [this.newTitle, this.newDescription];
  }

  makePreviewApplier(context: PreviewContext): ApplyPreviewsFuncType | undefined {
    const {treeMap} = context;

    const [bottom, top] = ends(this.foldRange);
    const topOfStack = treeMap.get(latestSuccessor(context, exactRevset(top.hash)));
    const children = topOfStack?.children ?? [];

    const func: ApplyPreviewsFuncType = tree => {
      if (tree.info.hash === latestSuccessor(context, exactRevset(bottom.hash))) {
        return {
          info: {
            ...bottom,
            date: new Date(),
            hash: getFoldRangeCommitHash(this.foldRange, /* isPreview */ true),
            title: this.newTitle,
            description: this.newDescription,
          },
          children,
          previewType: CommitPreview.FOLD_PREVIEW,
        };
      } else {
        return tree;
      }
    };
    return func;
  }

  makeOptimisticApplier(context: PreviewContext): ApplyPreviewsFuncType | undefined {
    const {treeMap} = context;

    const [bottom, top] = ends(this.foldRange);
    const topOfStack = treeMap.get(latestSuccessor(context, exactRevset(top.hash)));
    const children = topOfStack?.children ?? [];

    const func: ApplyPreviewsFuncType = tree => {
      if (tree.info.hash === latestSuccessor(context, exactRevset(bottom.hash))) {
        return {
          info: {
            ...bottom,
            date: new Date(),
            hash: getFoldRangeCommitHash(this.foldRange, /* isPreview */ false),
            title: this.newTitle,
            description: this.newDescription,
          },
          children,
          previewType: CommitPreview.FOLD,
        };
      } else {
        return tree;
      }
    };
    return func;
  }
}

export const FOLD_COMMIT_PREVIEW_HASH_PREFIX = 'OPTIMISTIC_FOLDED_PREVIEW_';
export const FOLD_COMMIT_OPTIMISTIC_HASH_PREFIX = 'OPTIMISTIC_FOLDED_';
export function getFoldRangeCommitHash(range: Array<CommitInfo>, isPreview: boolean): string {
  const [bottom, top] = ends(range);
  return (
    (isPreview ? FOLD_COMMIT_PREVIEW_HASH_PREFIX : FOLD_COMMIT_OPTIMISTIC_HASH_PREFIX) +
    `${bottom.hash}:${top.hash}`
  );
}