/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import type {Deferred} from 'shared/utils';

import {useCommand} from './ISLShortcuts';
import {Modal} from './Modal';
import {VSCodeButton} from '@vscode/webview-ui-toolkit/react';
import {atom, useRecoilState, useSetRecoilState} from 'recoil';
import {Icon} from 'shared/Icon';
import {defer} from 'shared/utils';

import './useModal.css';

type ModalConfig<T> =
  | {
      // hack: using 'confirm' mode requires T to be string.
      // The type inference goes wrong if we try to add this constraint directly to the `buttons` field.
      // By adding the constaint here, we get type checking that T is string in order to use this API.
      type: T extends string ? 'confirm' : never;
      message: React.ReactNode;
      buttons: ReadonlyArray<T>;
      /** Optional codicon to show next to the title */
      icon?: string;
      title: React.ReactNode;
    }
  | {
      type: 'custom';
      component: React.FC<{returnResultAndDismiss: (data: T) => void}>;
      /** Optional codicon to show next to the title */
      icon?: string;
      title: React.ReactNode;
    };
type ModalState<T> = {
  config: ModalConfig<T>;
  visible: boolean;
  deferred: Deferred<T | undefined>;
};

const modalState = atom<ModalState<unknown | string> | null>({
  key: 'modalState',
  default: null,
});

/** Wrapper around <Modal>, generated by `useModal()` hooks. */
export function ModalContainer() {
  const [modal, setModal] = useRecoilState(modalState);

  const dismiss = () => {
    if (modal?.visible) {
      modal.deferred.resolve(undefined);
      setModal({...modal, visible: false});
    }
  };

  useCommand('Escape', dismiss);

  if (modal?.visible !== true) {
    return null;
  }

  let content;
  if ((modal.config as ModalConfig<string>).type === 'confirm') {
    const config = modal.config as ModalConfig<string> & {type: 'confirm'};
    content = (
      <>
        <div id="use-modal-message">{config.message}</div>
        <div className="use-modal-buttons">
          {config.buttons.map(button => (
            <VSCodeButton
              appearance="secondary"
              onClick={() => {
                modal.deferred.resolve(button);
                setModal({...modal, visible: false});
              }}
              key={button}>
              {button}
            </VSCodeButton>
          ))}
        </div>
      </>
    );
  } else if (modal.config.type === 'custom') {
    const Component = modal.config.component;
    content = (
      <Component
        returnResultAndDismiss={data => {
          modal.deferred.resolve(data);
          setModal({...modal, visible: false});
        }}
      />
    );
  }

  return (
    <Modal
      width="500px"
      height="fit-content"
      className="use-modal"
      aria-labelledby="use-modal-title"
      aria-describedby="use-modal-message"
      dismiss={dismiss}>
      <div id="use-modal-title">
        {modal.config.icon != null ? <Icon icon={modal.config.icon} size="M" /> : null}
        {typeof modal.config.title === 'string' ? (
          <span>{modal.config.title}</span>
        ) : (
          modal.config.title
        )}
      </div>
      {content}
    </Modal>
  );
}

/**
 * Hook that provides a callback to show a modal with customizable behavior.
 * Modal has a dismiss button & dismisses on Escape keypress, thus you must always be able to handle
 * returning `undefined`.
 *
 * For now, we assume all uses of useOptionModal are triggerred directly from a user action.
 * If that's not the case, it would be possible to have multiple modals overlap.
 **/
export function useModal(): <T>(config: ModalConfig<T>) => Promise<T | undefined> {
  const setModal = useSetRecoilState(modalState);

  return <T,>(config: ModalConfig<T>) => {
    const deferred = defer<T | undefined>();
    // The API we provide is typed with T, but our recoil state only knows `unknown`, so we have to cast.
    // This is safe because only one modal is visible at a time, so we know the data type we created it with is what we'll get back.
    setModal({
      config: config as ModalConfig<unknown>,
      visible: true,
      deferred: deferred as Deferred<unknown | undefined>,
    });

    return deferred.promise as Promise<T>;
  };
}
