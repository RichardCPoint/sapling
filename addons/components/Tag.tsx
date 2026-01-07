/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import type {ReactNode} from 'react';
import type {ReactProps} from './utils';

import {Icon} from './Icon';
import * as stylex from '@stylexjs/stylex';

const styles = stylex.create({
  tag: {
    backgroundColor: 'var(--badge-background)',
    border: '1px solid var(--button-border)',
    borderRadius: 'var(--tag-corner-radius, 2px)',
    color: 'var(--badge-foreground)',
    padding: '2px 4px',
    fontFamily: 'var(--font-family)',
    fontSize: '11px',
    lineHeight: '16px',
    maxWidth: '150px',
  },
  flex: {
    display: 'inline-flex',
    gap: 'var(--halfpad)',
    alignItems: 'center',
    justifyContent: 'center',
  },
  icon: {
    display: 'block',
    flexShrink: 0,
  },
  text: {
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    minWidth: 0,
  },
});

export function Tag({
  xstyle,
  icon = null,
  children,
  ...rest
}: {
  children: ReactNode;
  icon?: null | React.ComponentProps<typeof Icon>['icon'];
  xstyle?: stylex.StyleXStyles;
} & ReactProps<HTMLSpanElement>) {
  if (icon != null) {
    return (
      <span {...stylex.props(styles.tag, styles.flex, xstyle)} {...rest}>
        <Icon size="S" icon={icon} {...stylex.props(styles.icon)} />
        <span {...stylex.props(styles.text)}>{children}</span>
      </span>
    );
  } else {
    return (
      <span {...stylex.props(styles.tag, styles.text, xstyle)} {...rest}>
        {children}
      </span>
    );
  }
}
