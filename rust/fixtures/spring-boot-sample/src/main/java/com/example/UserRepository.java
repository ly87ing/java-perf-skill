package com.example;

import org.springframework.stereotype.Repository;
import java.util.List;
import java.util.concurrent.ConcurrentHashMap;
import java.util.Map;

/**
 * 示例 Repository - 包含典型性能问题
 */
@Repository
public class UserRepository {

    // P0: static 无界缓存
    private static Map<Long, User> cache = new ConcurrentHashMap<>();

    public List<User> findAll() {
        // 模拟查询
        return List.of();
    }

    public User findById(Long id) {
        return cache.computeIfAbsent(id, this::queryFromDb);
    }

    public List<Order> findOrdersByUserId(Long userId) {
        // 这个方法在循环中被调用会导致 N+1
        return List.of();
    }

    private User queryFromDb(Long id) {
        // 模拟数据库查询
        return new User(id, "User " + id);
    }
}
