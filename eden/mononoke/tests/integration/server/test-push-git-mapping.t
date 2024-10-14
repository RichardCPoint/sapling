# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License found in the LICENSE file in the root
# directory of this source tree.

  $ . "${TEST_FIXTURES}/library.sh"

  $ setconfig push.edenapi=true
  $ INFINITEPUSH_NAMESPACE_REGEX='^scratch/.+$' POPULATE_GIT_MAPPING=1 EMIT_OBSMARKERS=1 BLOB_TYPE="blob_files" NON_GIT_TYPES=1 default_setup_drawdag
  A=aa53d24251ff3f54b1b2c29ae02826701b2abeb0079f1bb13b8434b54cd87675
  B=f8c75e41a0c4d29281df765f39de47bca1dcadfdc55ada4ccc2f6df567201658
  C=e32a1e342cdb1e38e88466b4c1a01ae9f410024017aa21dc0a1c5da6b3963bf2
  $ hg up -q master_bookmark
  $ cat >> .hg/hgrc <<EOF
  > [extensions]
  > remotenames=
  > infinitepush=
  > commitcloud=
  > [infinitepush]
  > server=False
  > branchpattern=re:scratch/.+
  > EOF

Push first commit to infiniepush
  $ touch file1
  $ hg ci -Aqm commit1 --extra hg-git-rename-source=git --extra convert_revision=1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a
  $ hg push -q -r . --to "scratch/123" --create

Check that mappings are empty
  $ get_bonsai_git_mapping | sort

Push another commit to master
  $ touch file2
  $ hg ci -Aqm commit2 --extra hg-git-rename-source=git --extra convert_revision=2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b
  $ hg push -q -r . --to master_bookmark --create

Check that mappings were populated
  $ get_bonsai_git_mapping | sort
  7AF229C8F6ED15A7C73DF5F9B2C2DE5CB588122E29F176397A3C52E41AB96791|1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A
  956B4E24CEDD3CBFFA0273C3750F771302699D4136331995B7AC5A68F8B3A73E|2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B

Push a commit to infinitepush, then move bookmark to it
  $ touch file3
  $ hg ci -Aqm commit1 --extra hg-git-rename-source=git --extra convert_revision=3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c
  $ hg push -q -r . --to "scratch/123" --create

  $ get_bonsai_git_mapping | sort
  7AF229C8F6ED15A7C73DF5F9B2C2DE5CB588122E29F176397A3C52E41AB96791|1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A
  956B4E24CEDD3CBFFA0273C3750F771302699D4136331995B7AC5A68F8B3A73E|2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B

  $ hg push -q -r . --to "master_bookmark"
  $ get_bonsai_git_mapping | sort
  7AF229C8F6ED15A7C73DF5F9B2C2DE5CB588122E29F176397A3C52E41AB96791|1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A
  956B4E24CEDD3CBFFA0273C3750F771302699D4136331995B7AC5A68F8B3A73E|2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B
  98C538B1514D847B3167A3EDC58891E1DD3753AD63775948AC6199D99D31664E|3C3C3C3C3C3C3C3C3C3C3C3C3C3C3C3C3C3C3C3C

Now push a commit to infinitepush, then force it to be public and then move bookmark to it
  $ touch file4
  $ hg ci -Aqm commit1 --extra hg-git-rename-source=git --extra convert_revision=4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d
  $ hg push -q -r . --to "scratch/123" --create

  $ mononoke_newadmin phases -R repo add-public -i $(hg log -r . -T '{node}\n') &> /dev/null

  $ hg push -q -r . --to "master_bookmark"
  $ get_bonsai_git_mapping | sort
  7AF229C8F6ED15A7C73DF5F9B2C2DE5CB588122E29F176397A3C52E41AB96791|1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A
  95472EDA8F395F39F6F7E860C0C3CEB75C4F94CB251DD186FBD2E70F96A702B6|4D4D4D4D4D4D4D4D4D4D4D4D4D4D4D4D4D4D4D4D
  956B4E24CEDD3CBFFA0273C3750F771302699D4136331995B7AC5A68F8B3A73E|2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B2B
  98C538B1514D847B3167A3EDC58891E1DD3753AD63775948AC6199D99D31664E|3C3C3C3C3C3C3C3C3C3C3C3C3C3C3C3C3C3C3C3C
