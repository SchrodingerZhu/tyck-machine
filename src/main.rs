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
    ApplyInfer {
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

#[derive(Collect)]
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
    pub fn direction(this: NodePtr<'a>, parent: NodePtr<'a>) -> Option<Direction> {
        let parent = parent.borrow();
        if let Some(child) = parent.get_child(Direction::Left) {
            if Gc::ptr_eq(child, this) {
                return Some(Direction::Left);
            }
        }
        if let Some(child) = parent.get_child(Direction::Right) {
            if Gc::ptr_eq(child, this) {
                return Some(Direction::Right);
            }
        }
        None
    }
    pub fn rotate(mc: &Mutation<'a>, mut node: NodePtr<'a>) {
        let Some(mut parent) = node.borrow().parent else {
            unreachable!("cannot rotate root")
        };
        let Some(dir) = Self::direction(node, parent) else {
            unreachable!("parent should be connected to child")
        };
        if let Some(grandparent) = parent.borrow().parent {
            // let Some(parent_dir)
            // grandparent.borrow_mut(mc).set_child(direction, child);
        }
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
