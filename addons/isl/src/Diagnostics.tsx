/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import type {UseUncommittedSelection} from './partialSelection';
import type {Diagnostic} from './types';

import {spacing} from '../../components/theme/tokens.stylex';
import serverAPI from './ClientToServerAPI';
import {Collapsable} from './Collapsable';
import {T, t} from './i18n';
import {localStorageBackedAtom, readAtom} from './jotaiUtils';
import foundPlatform from './platform';
import {uncommittedChangesWithPreviews} from './previews';
import {showModal} from './useModal';
import * as stylex from '@stylexjs/stylex';
import {Checkbox} from 'isl-components/Checkbox';
import {Column, Row} from 'isl-components/Flex';
import {Icon} from 'isl-components/Icon';
import {Subtle} from 'isl-components/Subtle';
import {useAtom} from 'jotai';
import {basename} from 'shared/utils';

export const shouldWarnAboutDiagnosticsAtom = localStorageBackedAtom<boolean>(
  'isl.warn-about-diagnostics',
  true,
);

const styles = stylex.create({
  diagnosticList: {
    paddingInline: spacing.double,
    paddingBlock: spacing.half,
    gap: 0,
  },
  nowrap: {
    whiteSpace: 'nowrap',
  },
  diagnosticRow: {
    maxWidth: 'max(400px, 80vw)',
    padding: spacing.half,
    cursor: 'pointer',
    ':hover': {
      backgroundColor: 'var(--hover-darken)',
    },
  },
  allDiagnostics: {
    maxHeight: '80vh',
    overflowY: 'scroll',
  },
  confirmCheckbox: {
    paddingTop: spacing.double,
  },
});

export async function confirmNoBlockingDiagnostics(
  selection: UseUncommittedSelection,
): Promise<boolean> {
  if (!readAtom(shouldWarnAboutDiagnosticsAtom)) {
    return true;
  }
  if (foundPlatform.platformName === 'vscode') {
    const allFiles = readAtom(uncommittedChangesWithPreviews);
    const selectedFiles = selection.isEverythingSelected()
      ? allFiles
      : allFiles.filter(file => selection.isFullyOrPartiallySelected(file.path));

    serverAPI.postMessage({
      type: 'platform/checkForDiagnostics',
      paths: selectedFiles.map(file => file.path),
    });
    const result = await serverAPI.nextMessageMatching('platform/gotDiagnostics', () => true);
    if (result.diagnostics.size > 0) {
      const allDiagnostics = [...result.diagnostics.values()];
      const totalErrors = allDiagnostics
        .map(value => value.filter(d => d.severity === 'error').length)
        .reduce((a, b) => a + b, 0);

      if (totalErrors > 0) {
        const buttons = [{label: 'Cancel'}, {label: 'Continue', primary: true}] as const;
        return (
          (await showModal({
            type: 'confirm',
            title: t('$num code issues found in selected files', {
              replace: {$num: String(totalErrors)},
            }),
            message: (
              <>
                <Column alignStart xstyle={styles.allDiagnostics}>
                  {[...result.diagnostics.entries()].map(([filepath, diagnostics]) => {
                    const sortedDiagnostics = [...diagnostics].sort((a, b) => {
                      return severityComparator(a) - severityComparator(b);
                    });
                    return (
                      <Column key={filepath} alignStart>
                        <Collapsable
                          startExpanded
                          title={
                            <Row>
                              <span>{basename(filepath)}</span>
                              <Subtle>{filepath}</Subtle>
                            </Row>
                          }>
                          <Column alignStart xstyle={styles.diagnosticList}>
                            {sortedDiagnostics.map(d => (
                              <Row
                                role="button"
                                tabIndex={0}
                                key={d.source}
                                xstyle={styles.diagnosticRow}
                                onClick={() => {
                                  foundPlatform.openFile(filepath, {line: d.range.startLine + 1});
                                }}>
                                {iconForDiagnostic(d)}
                                <span>{d.message}</span>
                                {d.source && (
                                  <Subtle {...stylex.props(styles.nowrap)}>
                                    {d.source}
                                    {d.code ? `(${d.code})` : null}
                                  </Subtle>
                                )}{' '}
                                <Subtle {...stylex.props(styles.nowrap)}>
                                  [Ln {d.range.startLine}, Col {d.range.startCol}]
                                </Subtle>
                              </Row>
                            ))}
                          </Column>
                        </Collapsable>
                      </Column>
                    );
                  })}
                </Column>
                <DontShowDiagnosticsConfirmationCheckbox />
              </>
            ),
            buttons,
          })) === buttons[1]
        );
      }
    }
  }
  return true;
}

function severityComparator(a: Diagnostic) {
  switch (a.severity) {
    case 'error':
      return 0;
    case 'warning':
      return 1;
    case 'info':
      return 2;
    case 'hint':
      return 3;
  }
}

function DontShowDiagnosticsConfirmationCheckbox() {
  const [shouldWarn, setShouldWarn] = useAtom(shouldWarnAboutDiagnosticsAtom);

  return (
    <Row xstyle={styles.confirmCheckbox}>
      <Checkbox checked={!shouldWarn} onChange={checked => setShouldWarn(!checked)}>
        <T>Don't ask again</T>
      </Checkbox>
    </Row>
  );
}

function iconForDiagnostic(d: Diagnostic): React.ReactNode {
  switch (d.severity) {
    case 'error':
      return <Icon icon="error" color="red" />;
    case 'warning':
      return <Icon icon="warning" color="yellow" />;
    case 'info':
      return <Icon icon="info" />;
    case 'hint':
      return <Icon icon="info" />;
  }
}
