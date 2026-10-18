import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/ui/group_pager.dart';

void main() {
  group('GroupPagerController', () {
    test('the group at an index is the group the user is looking at', () {
      final c = GroupPagerController(pageIds: ['apps', 'home', 'games']);
      for (var i = 0; i < 500; i++) {
        expect(
          c.groupIdAt(c.initialPage + i),
          ['apps', 'home', 'games'][i % 3],
        );
      }
      for (final g in ['apps', 'home', 'games']) {
        expect(c.groupIdAt(c.virtualIndexFor(g)), g);
        expect(c.currentGroupId(c.virtualIndexFor(g)), g);
      }
    });

    test('wrapping is symmetric in both directions', () {
      final c = GroupPagerController(pageIds: ['a', 'b', 'c']);
      expect(c.groupIdAt(c.initialPage - 1), 'c');
      expect(c.groupIdAt(c.initialPage + 3), 'a');
    });

    test('negative indices wrap to the end of the list', () {
      final c = GroupPagerController(pageIds: ['a', 'b', 'c']);
      expect(c.groupIdAt(-1), 'c');
      expect(c.groupIdAt(-4), 'c');
      expect(c.groupIdAt(-3), 'a');
    });

    test('a single page never wraps', () {
      final c = GroupPagerController(pageIds: ['only']);
      expect(c.groupIdAt(c.initialPage - 1), 'only');
      expect(c.groupIdAt(c.initialPage + 1), 'only');
    });

    for (final length in [1, 2, 3, 7, 10]) {
      test('initialPage is aligned to the first group for $length pages', () {
        final ids = [for (var i = 0; i < length; i++) 'g$i'];
        final c = GroupPagerController(pageIds: ids);
        expect(c.initialPage % length, 0);
        expect(c.groupIdAt(c.initialPage), ids.first);
        expect(c.initialPage, greaterThan(0));
        expect(
          c.initialPage,
          lessThanOrEqualTo(GroupPagerController.virtualPageCount ~/ 2),
        );
        expect(
          c.initialPage,
          greaterThan(GroupPagerController.virtualPageCount ~/ 2 - length),
        );
      });
    }

    test('initialPage follows the list when it changes', () {
      final c = GroupPagerController(pageIds: ['a', 'b', 'c']);
      c.pageIds = ['a', 'b', 'c', 'd', 'e', 'f', 'g'];
      expect(c.initialPage % 7, 0);
      expect(c.groupIdAt(c.initialPage), 'a');
    });

    test('currentPage starts at initialPage and follows onPageChanged', () {
      final c = GroupPagerController(pageIds: ['a', 'b', 'c']);
      expect(c.currentPage, c.initialPage);
      c.onPageChanged(c.initialPage + 1);
      expect(c.currentPage, c.initialPage + 1);
      expect(c.currentGroupId(c.currentPage), 'b');
    });

    test('currentGroupId has no side effect', () {
      final c = GroupPagerController(pageIds: ['a', 'b', 'c']);
      final before = c.currentPage;
      expect(c.currentGroupId(before + 2), 'c');
      expect(c.currentPage, before);
    });

    test('changing the page list keeps the current group when it survives', () {
      final c = GroupPagerController(pageIds: ['apps', 'home', 'games']);
      c.onPageChanged(c.initialPage + 1);
      expect(c.currentGroupId(c.currentPage), 'home');

      c.pageIds = ['games', 'new', 'home', 'apps'];
      expect(c.currentGroupId(c.currentPage), 'home');
      expect(c.groupIdAt(c.currentPage), 'home');
      expect(c.currentPage, c.virtualIndexFor('home'));

      c.pageIds = ['home'];
      expect(c.currentGroupId(c.currentPage), 'home');
    });

    test('keeps the current group from a page far from the anchor', () {
      final c = GroupPagerController(pageIds: ['a', 'b', 'c']);
      c.onPageChanged(c.initialPage - 7);
      expect(c.currentGroupId(c.currentPage), 'c');
      c.pageIds = ['c', 'a'];
      expect(c.currentGroupId(c.currentPage), 'c');
    });

    test('removing the current group resets to the first of the new list', () {
      final c = GroupPagerController(pageIds: ['apps', 'home', 'games']);
      c.onPageChanged(c.initialPage + 2);
      expect(c.currentGroupId(c.currentPage), 'games');

      c.pageIds = ['x', 'apps', 'home'];
      expect(c.currentPage, c.initialPage);
      expect(c.currentGroupId(c.currentPage), 'x');
    });

    test('an emptied page list does not throw', () {
      final c = GroupPagerController(pageIds: ['a', 'b']);
      c.onPageChanged(c.initialPage + 1);
      c.pageIds = [];
      expect(c.pageIds, isEmpty);
      expect(c.groupIdAt(c.initialPage), '');
      expect(c.groupIdAt(-1), '');
      expect(c.currentGroupId(c.currentPage), '');
      expect(c.virtualIndexFor('a'), -1);
      expect(c.initialPage, GroupPagerController.virtualPageCount ~/ 2);
    });

    test('a list that was empty and gains pages starts at the first group', () {
      final c = GroupPagerController(pageIds: []);
      expect(c.groupIdAt(0), '');
      c.pageIds = ['a', 'b', 'c'];
      expect(c.currentPage, c.initialPage);
      expect(c.currentGroupId(c.currentPage), 'a');
    });

    test('a group that is not in the list has index -1', () {
      final c = GroupPagerController(pageIds: ['a', 'b', 'c']);
      expect(c.virtualIndexFor('missing'), -1);
      expect(c.virtualIndexFor(''), -1);
    });

    test(
      'virtualIndexFor is the canonical index in the cycle at initialPage',
      () {
        final c = GroupPagerController(pageIds: ['a', 'b', 'c']);
        expect(c.virtualIndexFor('a'), c.initialPage);
        expect(c.virtualIndexFor('b'), c.initialPage + 1);
        expect(c.virtualIndexFor('c'), c.initialPage + 2);
      },
    );

    test('pageIds cannot be mutated from outside', () {
      final source = ['a', 'b', 'c'];
      final c = GroupPagerController(pageIds: source);
      source.add('d');
      source[0] = 'z';
      expect(c.pageIds, ['a', 'b', 'c']);
      expect(() => c.pageIds.add('x'), throwsUnsupportedError);
      expect(() => c.pageIds[0] = 'x', throwsUnsupportedError);

      final replacement = ['p', 'q'];
      c.pageIds = replacement;
      replacement.add('r');
      expect(c.pageIds, ['p', 'q']);
      expect(c.groupIdAt(c.initialPage + 2), 'p');
    });
  });

  group('GroupPager', () {
    Widget host(
      GroupPagerController controller, {
      ValueChanged<String>? onGroupChanged,
    }) {
      return MaterialApp(
        home: Scaffold(
          body: SizedBox(
            width: 400,
            height: 600,
            child: GroupPager(
              controller: controller,
              gridFor: (context, groupId) =>
                  Center(child: Text('group:$groupId')),
              onGroupChanged: onGroupChanged,
            ),
          ),
        ),
      );
    }

    Future<void> swipeToNext(WidgetTester tester) async {
      await tester.drag(find.byType(PageView), const Offset(-300, 0));
      await tester.pumpAndSettle();
    }

    Future<void> swipeToPrevious(WidgetTester tester) async {
      await tester.drag(find.byType(PageView), const Offset(300, 0));
      await tester.pumpAndSettle();
    }

    testWidgets('shows the first group initially', (tester) async {
      final c = GroupPagerController(pageIds: ['apps', 'home', 'games']);
      await tester.pumpWidget(host(c));
      expect(find.text('group:apps'), findsOneWidget);
      expect(find.text('group:home'), findsNothing);
      expect(find.text('group:games'), findsNothing);
    });

    testWidgets('dragging forward shows the next group, then the one after', (
      tester,
    ) async {
      final c = GroupPagerController(pageIds: ['apps', 'home', 'games']);
      await tester.pumpWidget(host(c));

      await swipeToNext(tester);
      expect(find.text('group:home'), findsOneWidget);
      expect(find.text('group:apps'), findsNothing);

      await swipeToNext(tester);
      expect(find.text('group:games'), findsOneWidget);
      expect(find.text('group:home'), findsNothing);

      await swipeToNext(tester);
      expect(find.text('group:apps'), findsOneWidget);
    });

    testWidgets('dragging backwards from the start wraps to the last group', (
      tester,
    ) async {
      final c = GroupPagerController(pageIds: ['apps', 'home', 'games']);
      await tester.pumpWidget(host(c));

      await swipeToPrevious(tester);
      expect(find.text('group:games'), findsOneWidget);
      expect(find.text('group:apps'), findsNothing);

      await swipeToPrevious(tester);
      expect(find.text('group:home'), findsOneWidget);
    });

    testWidgets('onGroupChanged reports the visible group and the controller '
        'tracks the page', (tester) async {
      final c = GroupPagerController(pageIds: ['apps', 'home', 'games']);
      final reported = <String>[];
      await tester.pumpWidget(host(c, onGroupChanged: reported.add));

      await swipeToNext(tester);
      expect(reported, ['home']);
      expect(c.currentPage, c.initialPage + 1);
      expect(c.currentGroupId(c.currentPage), 'home');

      await swipeToPrevious(tester);
      await swipeToPrevious(tester);
      expect(reported, ['home', 'apps', 'games']);
      expect(c.currentPage, c.initialPage - 1);
      expect(c.currentGroupId(c.currentPage), 'games');
    });

    testWidgets(
      'a changed page list keeps the visible group when it survives',
      (tester) async {
        final c = GroupPagerController(pageIds: ['apps', 'home', 'games']);
        await tester.pumpWidget(host(c));
        await swipeToNext(tester);
        expect(find.text('group:home'), findsOneWidget);

        c.pageIds = ['games', 'new', 'home', 'apps'];
        await tester.pumpWidget(host(c));
        await tester.pumpAndSettle();
        expect(find.text('group:home'), findsOneWidget);
        expect(c.currentGroupId(c.currentPage), 'home');

        await swipeToNext(tester);
        expect(find.text('group:apps'), findsOneWidget);
      },
    );

    testWidgets('a changed page list that drops the visible group shows the '
        'first group of the new list', (tester) async {
      final c = GroupPagerController(pageIds: ['apps', 'home', 'games']);
      await tester.pumpWidget(host(c));
      await swipeToNext(tester);
      expect(find.text('group:home'), findsOneWidget);

      c.pageIds = ['x', 'apps', 'games'];
      await tester.pumpWidget(host(c));
      await tester.pumpAndSettle();
      expect(find.text('group:x'), findsOneWidget);
      expect(find.text('group:home'), findsNothing);
    });

    testWidgets('an empty page list shows the empty group and does not throw', (
      tester,
    ) async {
      final c = GroupPagerController(pageIds: []);
      await tester.pumpWidget(host(c));
      expect(tester.takeException(), isNull);
      expect(find.text('group:'), findsOneWidget);

      await tester.drag(find.byType(PageView), const Offset(-300, 0));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(find.text('group:'), findsOneWidget);
    });

    testWidgets('pages appearing and disappearing are followed', (
      tester,
    ) async {
      final c = GroupPagerController(pageIds: []);
      await tester.pumpWidget(host(c));
      expect(find.text('group:'), findsOneWidget);

      c.pageIds = ['a', 'b'];
      await tester.pumpWidget(host(c));
      await tester.pumpAndSettle();
      expect(find.text('group:a'), findsOneWidget);

      await swipeToNext(tester);
      expect(find.text('group:b'), findsOneWidget);

      c.pageIds = [];
      await tester.pumpWidget(host(c));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(find.text('group:'), findsOneWidget);
    });

    testWidgets('a single group can wrap without duplicate-key errors', (
      tester,
    ) async {
      final c = GroupPagerController(pageIds: ['only']);
      await tester.pumpWidget(host(c));
      await swipeToNext(tester);
      await swipeToPrevious(tester);
      await swipeToPrevious(tester);
      expect(tester.takeException(), isNull);
      expect(find.text('group:only'), findsOneWidget);
    });

    testWidgets('rebuilding does not reset the scroll position', (
      tester,
    ) async {
      final c = GroupPagerController(pageIds: ['apps', 'home', 'games']);
      await tester.pumpWidget(host(c));
      await swipeToNext(tester);
      await tester.pumpWidget(host(c));
      await tester.pumpAndSettle();
      expect(find.text('group:home'), findsOneWidget);
    });
  });
}
