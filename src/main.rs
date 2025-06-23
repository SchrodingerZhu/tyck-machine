use std::{collections::HashMap, io::Empty, ops::Deref};

use gc_arena::{Arena, Collect, Gc, GcLock, GcRefLock, GcWeak, Mutation, RefLock, Rootable};

#[derive(Collect)]
#[collect(require_static)]
enum TyVarKind {
    Universal,
    Existential,
}

#[derive(Collect)]
#[collect(no_drop)]
enum Ty<'a> {
    Unit,
    Var(usize),
    Forall(String, Gc<'a, Self>),
    Arrow(Gc<'a, Self>, Gc<'a, Self>),
}

#[derive(Collect)]
#[collect(no_drop)]
enum Expr<'a> {
    Var(usize),
    App(Gc<'a, Self>, Gc<'a, Self>),
    Lam(String, Gc<'a, Self>),
    Let(String, Gc<'a, Self>, Gc<'a, Self>),
    Unit,
}   

#[derive(Collect)]
#[collect(no_drop)]
enum Judgment<'a> {
    Subtype {
        lhs: Gc<'a, Ty<'a>>,
        rhs: Gc<'a, Ty<'a>>,
    },
    Check {
        expr: Gc<'a, Expr<'a>>,
        ty: Gc<'a, Ty<'a>>,
    },
    Infer {
        expr: Gc<'a, Expr<'a>>,
        replace: usize,
        nested: Gc<'a, Self>,
    },
    AppInfer {
        func: Gc<'a, Ty<'a>>,
        expr: Gc<'a, Expr<'a>>,
        replace: usize,
        nested: Gc<'a, Self>,
    }
}

#[derive(Collect)]
#[collect(no_drop)]
enum WorkItem<'a>  {
    TyVarDecl(usize, TyVarKind),
    VarDecl(usize, Gc<'a, Ty<'a>>),
    Judgment(Gc<'a, Judgment<'a>>),
    Garbage,
}

#[derive(Collect, Clone, Copy)]
#[collect(require_static)]
enum Direction {
    Left,
    Right,
}

impl std::ops::Not for Direction {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

#[derive(Collect)]
#[collect(no_drop)]
struct SplayNode<'a> {
    parent: Option<NodePtr<'a>>,
    children: [Option<NodePtr<'a>>; 2],
}

type NodePtr<'a> = GcRefLock<'a, SplayNode<'a>>;

impl<'a> SplayNode<'a> {
    fn new(mc: &Mutation<'a>) -> GcRefLock<'a, Self> {
        Gc::new(mc,  RefLock::new(SplayNode {
            parent: None,
            children: [None, None],
        }))
    }
    pub fn get_child(&self, direction: Direction) -> Option<GcRefLock<'a, Self>> {
        self.children[direction as usize]
    }
    pub fn set_child(&mut self, direction: Direction, child: Option<GcRefLock<'a, Self>>) {
        self.children[direction as usize] = child;
    }
    pub fn direction(node: NodePtr<'a>, parent: NodePtr<'a>) -> Option<Direction> {
        let this = node.borrow();
        let parent = this.parent?.borrow();
        if let Some(child) = parent.get_child(Direction::Left) {
            if Gc::ptr_eq(child, node) {
                return Some(Direction::Left);
            }
        }
        if let Some(child) = parent.get_child(Direction::Right) {
            if Gc::ptr_eq(child, node) {
                return Some(Direction::Right);
            }
        }
        None
    }
    pub fn rotate(mc: &Mutation<'a>, node: NodePtr<'a>) {
        let parent = node.borrow().parent.expect("rotated node should have a parent");
        let grandparent = parent.borrow().parent;
        let dir = Self::direction(node, parent).expect("rotated node should be a child of its parent");
        if let Some(grandparent) = grandparent {
            let parent_dir = Self::direction(parent, grandparent).expect("parent should be a child of its parent");
            grandparent.borrow_mut(mc).set_child(parent_dir, Some(node));
        }
        node.borrow_mut(mc).parent = grandparent;

        let target_child = node.borrow().get_child(!dir);
        parent.borrow_mut(mc).set_child(dir, target_child);
        if let Some(target_child) = target_child {
            target_child.borrow_mut(mc).parent = Some(parent);
        }

        node.borrow_mut(mc).set_child(!dir, Some(parent));
        parent.borrow_mut(mc).parent = Some(node);
    }
    
}

// #[derive(Collect)]
// #[collect(no_drop)]
// struct TyckMachine<'a> {
//     worklist: GcIndexSet<WorkItem<'a>>,
//     ty_vars: HashMap<usize, TyVarKind>,
//     vars: HashMap<usize, Gc<'a, Ty<'a>>>,
//     replacement: HashMap<usize, Gc<'a, Ty<'a>>>,
// }

// type Machine<'a> = GcRefLock<'a, TyckMachine<'a>>;


fn main() {
    // let machine = Arena::<Rootable![Machine<'_>]>::new(|mc| {
    //     let machine = RefLock::new( 
    //         TyckMachine {
    //             worklist: GcIndexSet::default(),
    //             ty_vars: HashMap::new(),
    //             vars: HashMap::new(),
    //             replacement: HashMap::new(),
    //         },
    //     );
    //     Gc::new(mc, machine)
    // });
    // machine.muta
}
